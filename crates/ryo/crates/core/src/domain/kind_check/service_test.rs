//! The `kind_check` service's cases: a word a rule writes for a kind is checked against the kinds of the linked languages, and the
//! message names the word and the languages.

use super::service::{first_unknown, unknown_message};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::table_model::types::ParameterBinding;
use ryo_language::domain::relations::types::Relation;
use ryo_values::domain::descriptions::types::LanguageName;
use ryo_values::domain::descriptions::types::{KindVocabularies, KindVocabulary, KindWord};
use ryo_values::domain::plan_parts::types::{
    Applied, ColumnIndex, Comparison, Function, LiteralText, Operand, ParameterId, QueryLiteral,
    StepId, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep, StepScope};

/// What a row gives the check: how the kind is compared, with what, and the texts bound to the parameters.
type Given = (Test, Operand, Vec<ParameterBinding>);
/// What a row gives, as a function: the cases are constants.
type Input = fn() -> Given;
/// The message of the first word no language has a kind for; nothing when there is none.
type Value = &'static str;
type Failure = ();

fn plan_comparing_kind(test: Test, right: Operand) -> Plan {
    Plan {
        parameters: Vec::new(),
        steps: vec![PlanStep {
            operation: Operation::Select {
                from: ryo_values::domain::plan_parts::types::Input::Relation(Relation::SyntaxTree),
                comparisons: vec![Comparison {
                    test,
                    left: Operand::Column(ColumnIndex(1)),
                    right,
                }],
                kept: Vec::new(),
            },
            scope: StepScope::PerFile,
        }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
    }
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

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_kind_of_a_language_is_no_failure"),
        word: Word::Succeeds,
        input: || || (Test::Equal, text("source-file"), Vec::new()),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_kind_of_another_language_is_no_failure"),
        word: Word::Succeeds,
        input: || || (Test::Equal, text("table"), Vec::new()),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_kind_no_language_has_names_the_word_and_the_languages"),
        word: Word::Succeeds,
        input: || || (Test::Equal, text("function-declaration"), Vec::new()),
        expect: Expect::Value("kind `function-declaration` names no kind of rust, toml"),
    },
    Case {
        name: CaseName("a_pattern_that_matches_a_kind_is_no_failure"),
        word: Word::Succeeds,
        input: || || (Test::GlobAny, text("source-*"), Vec::new()),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_pattern_that_matches_no_kind_is_a_failure"),
        word: Word::Succeeds,
        input: || || (Test::GlobAny, text("class-*"), Vec::new()),
        expect: Expect::Value("kind `class-*` names no kind of rust, toml"),
    },
    Case {
        name: CaseName("the_words_a_list_parameter_is_bound_to_are_checked_each"),
        word: Word::Succeeds,
        input: || || (Test::In, parameter(), vec![texts(&["fn", "paragraph"])]),
        expect: Expect::Value("kind `paragraph` names no kind of rust, toml"),
    },
    Case {
        name: CaseName("a_list_parameter_of_kinds_that_exist_is_no_failure"),
        word: Word::Succeeds,
        input: || || (Test::In, parameter(), vec![texts(&["fn", "document"])]),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_not_equal_word_no_language_has_is_a_failure"),
        word: Word::Succeeds,
        input: || || (Test::NotEqual, text("nope"), Vec::new()),
        expect: Expect::Value("kind `nope` names no kind of rust, toml"),
    },
    Case {
        name: CaseName("a_not_in_list_with_a_kind_no_language_has_is_a_failure"),
        word: Word::Succeeds,
        input: || || (Test::NotIn, parameter(), vec![texts(&["fn", "nope"])]),
        expect: Expect::Value("kind `nope` names no kind of rust, toml"),
    },
    Case {
        name: CaseName("a_prefix_of_a_kind_is_no_failure"),
        word: Word::Succeeds,
        input: || || (Test::StartsWith, text("source-"), Vec::new()),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_prefix_no_kind_has_is_a_failure"),
        word: Word::Succeeds,
        input: || || (Test::StartsWith, text("class-"), Vec::new()),
        expect: Expect::Value("kind `class-` names no kind of rust, toml"),
    },
    Case {
        name: CaseName("a_suffix_of_a_kind_is_no_failure"),
        word: Word::Succeeds,
        input: || || (Test::EndsWith, text("-file"), Vec::new()),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_suffix_no_kind_has_is_a_failure"),
        word: Word::Succeeds,
        input: || || (Test::EndsWith, text("-declaration"), Vec::new()),
        expect: Expect::Value("kind `-declaration` names no kind of rust, toml"),
    },
    Case {
        name: CaseName("a_concat_of_constants_is_checked_as_the_text_it_makes"),
        word: Word::Succeeds,
        input: || {
            || {
                (
                    Test::Equal,
                    made(Function::Concat, &["totally-", "made-up"]),
                    Vec::new(),
                )
            }
        },
        expect: Expect::Value("kind `totally-made-up` names no kind of rust, toml"),
    },
    Case {
        name: CaseName("a_concat_that_makes_a_kind_is_no_failure"),
        word: Word::Succeeds,
        input: || {
            || {
                (
                    Test::Equal,
                    made(Function::Concat, &["source", "-file"]),
                    Vec::new(),
                )
            }
        },
        expect: Expect::Value(""),
    },
];

fn subject(input: Input) -> Value {
    let (test, right, bound) = input();
    let plan = plan_comparing_kind(test, right);
    let vocabularies = vocabularies();
    let message = first_unknown(&[plan], &[bound], &vocabularies)
        .map(|unknown| unknown_message(&unknown.word, &vocabularies).0)
        .unwrap_or_default();
    Box::leak(message)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
