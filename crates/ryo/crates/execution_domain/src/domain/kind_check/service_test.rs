//! The `kind_check` service's cases: a word a rule writes for a kind is checked against the kinds of the linked languages, and the
//! message names the word and the languages.

use std::borrow::Cow;

use super::service::{first_unknown, refusal, unknown_message};
use super::types::KindOrigin;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::descriptions::types::LanguageName;
use ryo_values::domain::descriptions::types::{KindVocabularies, KindVocabulary, KindWord};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{
    Applied, ColumnIndex, Function, LiteralText, Operand, ParameterId, PlanComparison,
    QueryLiteral, StepId, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// What a row gives the check: how the kind is compared, with what, and the texts bound to the parameters.
type Given = (Test, Operand, Vec<ParameterBinding>);
/// What a row asks of the kind check.
#[derive(Clone, Copy)]
enum Check {
    /// The first word no language has a kind for, the word compared as given, in a select and in a join, over `syntax_tree` and
    /// `semantic_syntax`.
    Word(fn() -> Given),
    /// The kinds asked by text that reads as a number, compared by this ordering (one that does not name a kind), in a select and
    /// in a join.
    NumericText(Test),
    /// The refusal of a word compared for equality in a plan whose only comparison origin is recorded at this source.
    Origin(&'static str, &'static str),
}
/// What the kind check answers.
#[derive(Debug, PartialEq)]
enum Answer {
    /// The message of the first word no language has a kind for (empty when there is none), and whether every relation and site
    /// gives the same message.
    Message {
        text: Cow<'static, str>,
        agrees: bool,
    },
    /// How many kinds were asked.
    Asked(usize),
    /// Whether a refusal keeps the origin of its own plan, and its message; none when the word is refused by nothing.
    Refused(Option<(bool, Cow<'static, str>)>),
}
type Input = Check;
type Value = Answer;
type Failure = std::convert::Infallible;

/// Both places where a plan can constrain the kind of a source relation.
#[derive(Clone, Copy)]
enum KindSite {
    /// A direct relation selection.
    Select,
    /// A filter on the right relation of a join.
    Join,
}

fn plan_comparing_kind(test: Test, right: Operand, relation: Relation, site: KindSite) -> Plan {
    let source = ryo_values::domain::plan_parts::types::Input::Relation(relation);
    let comparisons = vec![PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(1)),
        right,
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }];
    let operation = match site {
        KindSite::Select => Operation::Select {
            from: source,
            comparisons,
            kept: Vec::new(),
        },
        KindSite::Join => Operation::Join {
            left: ryo_values::domain::plan_parts::types::Input::Relation(Relation::SyntaxTree),
            right: source,
            left_on: Vec::new(),
            right_on: Vec::new(),
            output: Vec::new(),
            filter: comparisons,
            view: Vec::new(),

            reach: None,
        },
    };
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: vec![PlanStep {
            operation,
            scope: StepScope::PerFile,
        }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
        comparison_origins: Vec::new(),
    }
}

fn kinds_asked_of_numeric_text(test: Test) -> Value {
    Answer::Asked(
        [KindSite::Select, KindSite::Join]
            .into_iter()
            .map(|site| {
                let plan = plan_comparing_kind(
                    test,
                    Operand::Literal(QueryLiteral::Text(LiteralText("1.0".into()))),
                    Relation::SyntaxTree,
                    site,
                );
                super::service::kinds_asked(&plan, &[]).len()
            })
            .sum(),
    )
}

fn refused_with_origin(word: &str, source: &str) -> Value {
    use ryo_values::domain::plan_parts::types::{ComparisonOrigin, ComparisonOriginId};
    use ryo_values::domain::query_tokens::types::{
        ByteOffset, QueryOrigin, QuerySourceId, QuerySpan,
    };
    let origin = QueryOrigin {
        source_id: QuerySourceId(source.into()),
        span: QuerySpan {
            start: ByteOffset(12),
            end: ByteOffset(31),
        },
        trace: Vec::new(),
    };
    let mut plan = plan_comparing_kind(
        Test::Equal,
        text(word),
        Relation::SyntaxTree,
        KindSite::Select,
    );
    plan.comparison_origins.push(origin.clone());
    if let Operation::Select { comparisons, .. } = &mut plan.steps[0].operation {
        comparisons[0].origin = ComparisonOrigin::Recorded(ComparisonOriginId(0));
    }
    Answer::Refused(
        refusal(&plan, &KindVocabularies(Vec::new()), matcher()).map(|refused| {
            (
                refused.origin == KindOrigin::Recorded(origin),
                Cow::Owned(refused.message.0.into()),
            )
        }),
    )
}

/// The matching semantics the check runs through.
fn matcher() -> ryo_operators::domain::patterns::types::PatternMatcher<'static> {
    ryo_operators::domain::patterns::types::PatternMatcher(
        &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
    )
}

fn vocabularies() -> KindVocabularies {
    let language = |name: &str, kinds: &[&str]| KindVocabulary {
        language: LanguageName(name.into()),
        kinds: kinds
            .iter()
            .map(|kind| KindWord(Box::from(*kind)))
            .collect(),
    };
    KindVocabularies(vec![
        language("rust", &["source-file", "fn"]),
        language("toml", &["document", "table"]),
    ])
}

fn text(text: &str) -> Operand {
    Operand::Literal(QueryLiteral::Text(LiteralText(text.into())))
}

fn made(function: Function, arguments: &[&str]) -> Operand {
    Operand::Function(Applied {
        function,
        arguments: arguments.iter().map(|argument| text(argument)).collect(),
    })
}

fn parameter() -> Operand {
    Operand::Parameter(ParameterId(0))
}

fn texts(each: &[&str]) -> ParameterBinding {
    ParameterBinding::Texts(
        each.iter()
            .map(|word| LiteralText((*word).into()))
            .collect(),
    )
}

/// The message every relation and site agrees on.
const fn said(text: &'static str) -> Answer {
    Answer::Message {
        text: Cow::Borrowed(text),
        agrees: true,
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_kind_of_a_language_is_no_failure"),
        word: Word::Succeeds,
        input: || Check::Word(|| (Test::Equal, text("source-file"), Vec::new())),
        expect: Expect::Value(said("")),
    },
    Case {
        name: CaseName("a_kind_of_another_language_is_no_failure"),
        word: Word::Succeeds,
        input: || Check::Word(|| (Test::Equal, text("table"), Vec::new())),
        expect: Expect::Value(said("")),
    },
    Case {
        name: CaseName("a_kind_no_language_has_names_the_word_and_the_languages"),
        word: Word::Succeeds,
        input: || Check::Word(|| (Test::Equal, text("function-declaration"), Vec::new())),
        expect: Expect::Value(said(
            "kind `function-declaration` names no kind of rust, toml",
        )),
    },
    Case {
        name: CaseName("a_pattern_that_matches_a_kind_is_no_failure"),
        word: Word::Succeeds,
        input: || Check::Word(|| (Test::GlobAny, text("source-*"), Vec::new())),
        expect: Expect::Value(said("")),
    },
    Case {
        name: CaseName("a_pattern_that_matches_no_kind_is_a_failure"),
        word: Word::Succeeds,
        input: || Check::Word(|| (Test::GlobAny, text("class-*"), Vec::new())),
        expect: Expect::Value(said("kind `class-*` names no kind of rust, toml")),
    },
    Case {
        name: CaseName("the_words_a_list_parameter_is_bound_to_are_checked_each"),
        word: Word::Succeeds,
        input: || Check::Word(|| (Test::In, parameter(), vec![texts(&["fn", "paragraph"])])),
        expect: Expect::Value(said("kind `paragraph` names no kind of rust, toml")),
    },
    Case {
        name: CaseName("a_list_parameter_of_kinds_that_exist_is_no_failure"),
        word: Word::Succeeds,
        input: || Check::Word(|| (Test::In, parameter(), vec![texts(&["fn", "document"])])),
        expect: Expect::Value(said("")),
    },
    Case {
        name: CaseName("a_not_equal_word_no_language_has_is_a_failure"),
        word: Word::Succeeds,
        input: || Check::Word(|| (Test::NotEqual, text("nope"), Vec::new())),
        expect: Expect::Value(said("kind `nope` names no kind of rust, toml")),
    },
    Case {
        name: CaseName("a_not_in_list_with_a_kind_no_language_has_is_a_failure"),
        word: Word::Succeeds,
        input: || Check::Word(|| (Test::NotIn, parameter(), vec![texts(&["fn", "nope"])])),
        expect: Expect::Value(said("kind `nope` names no kind of rust, toml")),
    },
    Case {
        name: CaseName("a_prefix_of_a_kind_is_no_failure"),
        word: Word::Succeeds,
        input: || Check::Word(|| (Test::StartsWith, text("source-"), Vec::new())),
        expect: Expect::Value(said("")),
    },
    Case {
        name: CaseName("a_prefix_no_kind_has_is_a_failure"),
        word: Word::Succeeds,
        input: || Check::Word(|| (Test::StartsWith, text("class-"), Vec::new())),
        expect: Expect::Value(said("kind `class-` names no kind of rust, toml")),
    },
    Case {
        name: CaseName("a_suffix_of_a_kind_is_no_failure"),
        word: Word::Succeeds,
        input: || Check::Word(|| (Test::EndsWith, text("-file"), Vec::new())),
        expect: Expect::Value(said("")),
    },
    Case {
        name: CaseName("a_suffix_no_kind_has_is_a_failure"),
        word: Word::Succeeds,
        input: || Check::Word(|| (Test::EndsWith, text("-declaration"), Vec::new())),
        expect: Expect::Value(said("kind `-declaration` names no kind of rust, toml")),
    },
    Case {
        name: CaseName("a_concat_of_constants_is_checked_as_the_text_it_makes"),
        word: Word::Succeeds,
        input: || {
            Check::Word(|| {
                (
                    Test::Equal,
                    made(Function::Concat, &["totally-", "made-up"]),
                    Vec::new(),
                )
            })
        },
        expect: Expect::Value(said("kind `totally-made-up` names no kind of rust, toml")),
    },
    Case {
        name: CaseName("a_concat_that_makes_a_kind_is_no_failure"),
        word: Word::Succeeds,
        input: || {
            Check::Word(|| {
                (
                    Test::Equal,
                    made(Function::Concat, &["source", "-file"]),
                    Vec::new(),
                )
            })
        },
        expect: Expect::Value(said("")),
    },
    Case {
        name: CaseName("numeric_text_is_not_mistaken_for_a_declared_syntax_kind"),
        word: Word::Succeeds,
        input: || Check::NumericText(Test::NaturalLessThan),
        expect: Expect::Value(Answer::Asked(0)),
    },
    Case {
        name: CaseName("numeric_text_compared_numerically_names_no_kind"),
        word: Word::Succeeds,
        input: || Check::NumericText(Test::NumericGreaterThanOrEqual),
        expect: Expect::Value(Answer::Asked(0)),
    },
    Case {
        name: CaseName("numeric_text_compared_by_successor_names_no_kind"),
        word: Word::Succeeds,
        input: || Check::NumericText(Test::IntegerSuccessor),
        expect: Expect::Value(Answer::Asked(0)),
    },
    Case {
        name: CaseName("an_unknown_kind_keeps_the_origin_from_its_own_plan_table"),
        word: Word::Succeeds,
        input: || Check::Origin("not-a-kind", "fragment/filter"),
        expect: Expect::Value(Answer::Refused(Some((
            true,
            Cow::Borrowed("kind `not-a-kind` names no kind of "),
        )))),
    },
    Case {
        name: CaseName("an_unknown_kind_of_another_plan_keeps_that_plan_s_origin"),
        word: Word::Succeeds,
        input: || Check::Origin("also-not-a-kind", "other/filter"),
        expect: Expect::Value(Answer::Refused(Some((
            true,
            Cow::Borrowed("kind `also-not-a-kind` names no kind of "),
        )))),
    },
];

fn message(given: fn() -> Given, relation: Relation, site: KindSite) -> String {
    let (test, right, bound) = given();
    let plan = plan_comparing_kind(test, right, relation, site);
    let vocabularies = vocabularies();
    first_unknown(&[plan], &[bound], &vocabularies, matcher())
        .map(|unknown| {
            unknown_message(&unknown.word, &vocabularies)
                .message
                .0
                .into()
        })
        .unwrap_or_default()
}

fn subject(check: Input) -> Result<Value, Failure> {
    Ok(match check {
        Check::Word(given) => {
            let expected = message(given, Relation::SyntaxTree, KindSite::Select);
            let agrees = [Relation::SyntaxTree, Relation::SemanticSyntax]
                .into_iter()
                .flat_map(|relation| {
                    [KindSite::Select, KindSite::Join].map(|site| (relation, site))
                })
                .all(|(relation, site)| message(given, relation, site) == expected);
            Answer::Message {
                text: Cow::Owned(expected),
                agrees,
            }
        }
        Check::NumericText(test) => kinds_asked_of_numeric_text(test),
        Check::Origin(word, source) => refused_with_origin(word, source),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
