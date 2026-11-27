//! The `kind_check` service: the words a lowered query writes for a kind (a comparison of the kind column of the syntax relations), and
//! whether a linked language has a kind each names. The one kind check: `query validate` and a run both ask it of the plan that runs.

use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase as _;
use ryo_matching::inbound::use_cases::matching::types::{GlobPattern, MatchOutcome, RegexSource, SearchText};
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
use ryo_values::domain::descriptions::types::KindVocabularies;
use ryo_values::domain::finish_plans::expressions::types::{Comparator, PlanConstant, PlanExpression, SourceAlias};
use ryo_values::domain::finish_plans::types::{FinishPlan, FinishStep, SelectStep};
use ryo_values::domain::plan_parts::types::Test;

/// The relations whose kind column holds the kinds of a language's nodes and tokens.
const KINDED: [&str; 2] = ["syntax-tree", "semantic-syntax"];
const KIND: &str = "kind";

/// The words of `plan` that are compared with the kind of a node or a token, and how.
#[must_use]
pub fn asked(plan: &FinishPlan) -> Vec<(Test, String)> {
    let mut found = Vec::new();
    for step in &plan.steps {
        let FinishStep::Select(select) = step else { continue };
        let kinded: Vec<&SourceAlias> = select
            .sources
            .iter()
            .filter(|source| matches!(plan.steps.get(source.step.0), Some(FinishStep::Base(name)) if KINDED.contains(&&*name.0)))
            .map(|source| &source.alias)
            .collect();
        if kinded.is_empty() {
            continue;
        }
        if let ryo_values::domain::finish_plans::types::RowCondition::Tested(condition) = &select.condition {
            walk(condition, select, &kinded, &mut found);
        }
    }
    found
}

/// Whether `expression` reads the kind column of one of the sources that have one.
fn is_kind(expression: &PlanExpression, select: &SelectStep, kinded: &[&SourceAlias]) -> bool {
    let PlanExpression::Column(read) = expression else { return false };
    &*read.column.0 == KIND
        && match &read.alias {
            alias @ SourceAlias::Written(_) => kinded.contains(&alias),
            SourceAlias::Unwritten => select.sources.len() == 1,
        }
}

/// The texts a constant or a list of them holds.
fn texts(expression: &PlanExpression, out: &mut Vec<String>) {
    match expression {
        PlanExpression::Literal(constant) => constant_texts(constant, out),
        PlanExpression::List(items) => items.iter().for_each(|item| texts(item, out)),
        _ => {}
    }
}

fn constant_texts(constant: &PlanConstant, out: &mut Vec<String>) {
    match constant {
        PlanConstant::Text(text) => out.push(text.0.to_string()),
        PlanConstant::List(items) => items.iter().for_each(|item| constant_texts(item, out)),
        _ => {}
    }
}

fn walk(expression: &PlanExpression, select: &SelectStep, kinded: &[&SourceAlias], found: &mut Vec<(Test, String)>) {
    let mut ask = |test: Test, value: &PlanExpression, found: &mut Vec<(Test, String)>| {
        let mut words = Vec::new();
        texts(value, &mut words);
        found.extend(words.into_iter().map(|word| (test, word)));
    };
    match expression {
        PlanExpression::Compare(comparison) => {
            let test = match comparison.comparator {
                Comparator::Equal => Test::Equal,
                Comparator::NotEqual => Test::NotEqual,
                _ => return,
            };
            if is_kind(&comparison.left, select, kinded) {
                ask(test, &comparison.right, found);
            } else if is_kind(&comparison.right, select, kinded) {
                ask(test, &comparison.left, found);
            }
        }
        PlanExpression::In(pair) | PlanExpression::NotIn(pair) => {
            if is_kind(&pair.left, select, kinded) {
                ask(Test::In, &pair.right, found);
            }
        }
        PlanExpression::Call(call) => {
            let test = match &*call.function.0 {
                "glob_any" => Test::GlobAny,
                "starts_with" => Test::StartsWith,
                "ends_with" => Test::EndsWith,
                "matches" => Test::Matches,
                _ => return,
            };
            if let [value, pattern] = call.arguments.as_slice()
                && is_kind(value, select, kinded)
            {
                ask(test, pattern, found);
            }
        }
        PlanExpression::And(pair) | PlanExpression::Or(pair) => {
            walk(&pair.left, select, kinded, found);
            walk(&pair.right, select, kinded, found);
        }
        PlanExpression::Not(inner) => walk(inner, select, kinded, found),
        PlanExpression::Column(_) | PlanExpression::Literal(_) | PlanExpression::Number(_) | PlanExpression::List(_) => {}
    }
}

/// Whether some language of `vocabularies` has a kind the word names, compared as `test` says: a name is equal to it, a prefix or a
/// suffix of it is, a pattern matches it.
#[must_use]
pub fn names_kind(test: Test, word: &str, vocabularies: &KindVocabularies) -> bool {
    let matching = MatchingUseCaseImpl::new();
    let regex = (test == Test::Matches).then(|| matching.compile_regex(RegexSource(word)));
    vocabularies.0.iter().any(|vocabulary| {
        vocabulary.kinds.iter().any(|kind| {
            let kind = &*kind.0;
            match test {
                Test::GlobAny | Test::NotGlobAny => matching.matches_text_glob(GlobPattern(word), SearchText(kind)) == MatchOutcome::Matches,
                Test::StartsWith => kind.starts_with(word),
                Test::EndsWith => kind.ends_with(word),
                Test::Matches => regex
                    .as_ref()
                    .and_then(|compiled| compiled.as_ref().ok())
                    .is_some_and(|compiled| matching.matches_regex(compiled, SearchText(kind)) == MatchOutcome::Matches),
                _ => kind == word,
            }
        })
    })
}

/// The first word `plan` writes for a kind that no language of `vocabularies` has a kind for, or nothing.
#[must_use]
pub fn unknown_word(plan: &FinishPlan, vocabularies: &KindVocabularies) -> Option<String> {
    asked(plan).into_iter().find(|(test, word)| !names_kind(*test, word, vocabularies)).map(|(_, word)| word)
}

/// What is said of a word that names no kind: the word and the languages that were looked in.
#[must_use]
pub fn unknown_message(word: &str, vocabularies: &KindVocabularies) -> String {
    let languages: Vec<&str> = vocabularies.0.iter().map(|vocabulary| &*vocabulary.language.0).collect();
    format!("kind `{word}` names no kind of {}", languages.join(", "))
}
