//! The `kind_check` service: free functions named by verb that find the words a plan writes for a kind and say whether any language has
//! a kind they name, with no IO and no ports.

use crate::domain::functions::service::apply;
use crate::domain::functions::types::MadeText;
use crate::domain::kind_check::constants::LANGUAGE_SEPARATOR;
use crate::domain::kind_check::types::{
    AskedKind, KindMatch, KindMessage, KindOrigin, KindRefusal, LanguageList, RulePlace,
    UnknownKind,
};
use crate::domain::kind_sets::service::satisfies;
use crate::domain::matching::types::Matcher;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::types::TextView;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::descriptions::types::{KindVocabularies, KindWord};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input, Operand, ParameterId, PlanComparison, QueryLiteral, Test,
};
use ryo_values::domain::plans::types::{Operation, Plan};
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// The place of the kind among the columns of the syntax tree.
const KIND_COLUMN: ColumnIndex = ColumnIndex(1);

/// The words `plan` compares the kind of a node or a token with, once the parameters are bound: a literal, or the texts a parameter is
/// bound to.
#[must_use]
pub(crate) fn kinds_asked(plan: &Plan, bound: &[ParameterBinding]) -> Vec<AskedKind> {
    let mut asked = Vec::new();
    for step in &plan.steps {
        if let Operation::Select {
            from, comparisons, ..
        } = &step.operation
            && matches!(
                from,
                Input::Relation(Relation::SyntaxTree | Relation::SemanticSyntax)
            )
        {
            asked_in(comparisons, &plan.comparison_origins, bound, &mut asked);
        }
        if let Operation::Join { right, filter, .. } = &step.operation
            && matches!(
                right,
                Input::Relation(Relation::SyntaxTree | Relation::SemanticSyntax)
            )
        {
            asked_in(filter, &plan.comparison_origins, bound, &mut asked);
        }
    }
    asked
}

/// The words of the comparisons of the kind column, added to `asked`.
fn asked_in(
    comparisons: &[PlanComparison],
    origins: &[ryo_values::domain::query_tokens::types::QueryOrigin],
    bound: &[ParameterBinding],
    asked: &mut Vec<AskedKind>,
) {
    for comparison in comparisons {
        if comparison.left != Operand::Column(KIND_COLUMN) {
            continue;
        }
        // Numeric and natural ordering do not assert that its text operand names a syntax kind.
        if matches!(
            comparison.test,
            Test::NaturalLessThan | Test::NumericGreaterThanOrEqual | Test::IntegerSuccessor
        ) {
            continue;
        }
        let test = comparison.test;
        // The words the right side names: a literal text, the text or texts a parameter is bound to, or the text a function of
        // constants makes.
        let given = if let Operand::Parameter(ParameterId(id)) = &comparison.right {
            bound.get(usize::try_from(*id).unwrap_or(usize::MAX))
        } else {
            None
        };
        let many = if let Some(ParameterBinding::Texts(texts)) = given {
            texts.as_slice()
        } else {
            &[]
        };
        let words: Vec<String> = constant_text(&comparison.right, bound)
            .into_iter()
            .map(|made| made.0)
            .chain(many.iter().map(|text| text.0.to_string()))
            .collect();
        let origin = match comparison.origin {
            ryo_values::domain::plan_parts::types::ComparisonOrigin::Recorded(id) => origins
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .cloned()
                .map_or(KindOrigin::Unrecorded, KindOrigin::Recorded),
            ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded => {
                KindOrigin::Unrecorded
            }
        };
        asked.extend(words.into_iter().map(|word| AskedKind {
            test,
            word: KindWord(word.into()),
            origin: origin.clone(),
        }));
    }
}

/// The text `operand` is when the query is compiled: a literal text, the text a parameter is bound to, or the text a function makes of
/// operands that are such texts. Nothing when it depends on a row.
fn constant_text(operand: &Operand, bound: &[ParameterBinding]) -> Option<MadeText> {
    match operand {
        Operand::Literal(QueryLiteral::Text(text)) => Some(MadeText(text.0.to_string())),
        Operand::Parameter(ParameterId(id)) => {
            match bound.get(usize::try_from(*id).unwrap_or(usize::MAX)) {
                Some(ParameterBinding::Text(text)) => Some(MadeText(text.0.to_string())),
                _ => None,
            }
        }
        Operand::Function(applied) => {
            let arguments = applied
                .arguments
                .iter()
                .map(|argument| constant_text(argument, bound))
                .collect::<Option<Vec<_>>>()?;
            let mut made = MadeText::default();
            apply(applied.function, &arguments, &mut made);
            Some(made)
        }
        Operand::Column(_) | Operand::Literal(QueryLiteral::Number(_)) => None,
    }
}

/// Whether a kind of some language is the word `asked` names: a name is equal to it, a pattern matches it.
#[must_use]
pub(crate) fn names_a_kind(
    asked: &AskedKind,
    vocabularies: &KindVocabularies,
    matcher: Matcher<'_>,
) -> KindMatch {
    // A pattern, a prefix and a suffix are checked as the query compares them: against the kinds, not as an exact kind.
    let test = match asked.test {
        Test::GlobAny | Test::NotGlobAny => Test::GlobAny,
        Test::StartsWith => Test::StartsWith,
        Test::EndsWith => Test::EndsWith,
        Test::Matches => Test::Matches,
        Test::Equal
        | Test::NotEqual
        | Test::LessThan
        | Test::LessThanOrEqual
        | Test::GreaterThan
        | Test::GreaterThanOrEqual
        | Test::NaturalLessThan
        | Test::FloatTextGreaterThanOrEqual
        | Test::NumericGreaterThanOrEqual
        | Test::IntegerSuccessor
        | Test::In
        | Test::NotIn => Test::Equal,
    };
    let named = vocabularies.0.iter().any(|vocabulary| {
        vocabulary.kinds.iter().any(|kind| {
            satisfies(test, TextView(&kind.0), TextView(&asked.word.0), matcher) == Verdict::Holds
        })
    });
    if named {
        KindMatch::Names
    } else {
        KindMatch::Nothing
    }
}

/// The languages whose kinds were looked in, as a phrase names them: `first, second`.
#[must_use]
pub(crate) fn languages_of(vocabularies: &KindVocabularies) -> LanguageList {
    LanguageList(
        vocabularies
            .0
            .iter()
            .map(|vocabulary| &*vocabulary.language.0)
            .collect::<Vec<_>>()
            .join(LANGUAGE_SEPARATOR)
            .into(),
    )
}

/// The first word a rule writes for a kind that no linked language has a kind for.
#[must_use]
pub(crate) fn first_unknown(
    plans: &[Plan],
    bindings: &[Vec<ParameterBinding>],
    vocabularies: &KindVocabularies,
    matcher: Matcher<'_>,
) -> Option<UnknownKind> {
    plans.iter().enumerate().find_map(|(at, plan)| {
        let bound = bindings.get(at).map_or(&[][..], Vec::as_slice);
        kinds_asked(plan, bound)
            .into_iter()
            .find(|asked| names_a_kind(asked, vocabularies, matcher) == KindMatch::Nothing)
            .map(|asked| UnknownKind {
                rule: RulePlace(at),
                word: asked.word,
                origin: asked.origin,
            })
    })
}

/// What is said of a word that names no kind: the word and the languages that were looked in.
#[must_use]
pub(crate) fn unknown_message(word: &KindWord, vocabularies: &KindVocabularies) -> KindRefusal {
    KindRefusal {
        message: KindMessage(
            format!(
                "kind `{}` names no kind of {}",
                word.0,
                languages_of(vocabularies).0
            )
            .into(),
        ),
        origin: KindOrigin::Unrecorded,
    }
}

/// Why a query refuses to name a kind: the message for the first word it writes for a kind that no language of `vocabularies` has,
/// or nothing when every word names one. Words that depend on a parameter are not known before the run and are not asked.
#[must_use]
pub fn refusal(
    plan: &Plan,
    vocabularies: &KindVocabularies,
    matcher: Matcher<'_>,
) -> Option<KindRefusal> {
    first_unknown(std::slice::from_ref(plan), &[Vec::new()], vocabularies, matcher).map(|unknown| {
        let mut refusal = unknown_message(&unknown.word, vocabularies);
        refusal.origin = unknown.origin;
        refusal
    })
}
