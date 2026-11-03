//! The `kind_check` service's cases: a word a rule writes for a kind is checked against the kinds of the linked languages, and the
//! message names the word and the languages.

use super::service::{first_unknown, refusal, unknown_message};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::table_model::types::ParameterBinding;
use ryo_relations::domain::relations::types::Relation;
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

/// Both places where a plan can constrain the kind of a source relation.
#[derive(Clone, Copy)]
enum KindSite {
    /// A direct relation selection.
    Select,
    /// A filter on the right relation of a join.
    Join,
}

#[test]
fn numeric_text_is_not_mistaken_for_a_declared_syntax_kind() {
    for test in [
        Test::NaturalLessThan,
        Test::NumericGreaterThanOrEqual,
        Test::IntegerSuccessor,
    ] {
        for site in [KindSite::Select, KindSite::Join] {
            let plan = plan_comparing_kind(
                test,
                Operand::Literal(QueryLiteral::Text(LiteralText("1.0".into()))),
                Relation::SyntaxTree,
                site,
            );
            assert!(super::service::kinds_asked(&plan, &[]).is_empty());
        }
    }
}

fn plan_comparing_kind(test: Test, right: Operand, relation: Relation, site: KindSite) -> Plan {
    let source = ryo_values::domain::plan_parts::types::Input::Relation(relation);
    let comparisons = vec![Comparison {
        test,
        left: Operand::Column(ColumnIndex(1)),
        right,
        origin: None,
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

#[test]
fn an_unknown_kind_keeps_the_origin_from_its_own_plan_table() {
    use ryo_values::domain::plan_parts::types::ComparisonOriginId;
    use ryo_values::domain::program::types::{ByteOffset, QueryOrigin, QuerySourceId, QuerySpan};

    let origin = QueryOrigin {
        source_id: QuerySourceId("fragment/filter".into()),
        span: QuerySpan {
            start: ByteOffset(12),
            end: ByteOffset(31),
        },
        trace: Vec::new(),
    };
    let mut plan = plan_comparing_kind(
        Test::Equal,
        text("not-a-kind"),
        Relation::SyntaxTree,
        KindSite::Select,
    );
    plan.comparison_origins.push(origin.clone());
    let Operation::Select { comparisons, .. } = &mut plan.steps[0].operation else {
        panic!("the test plan uses a select");
    };
    comparisons[0].origin = Some(ComparisonOriginId(0));

    let mut other_origin = origin.clone();
    other_origin.source_id = QuerySourceId("other/filter".into());
    let mut other_plan = plan_comparing_kind(
        Test::Equal,
        text("also-not-a-kind"),
        Relation::SyntaxTree,
        KindSite::Select,
    );
    other_plan.comparison_origins.push(other_origin.clone());
    let Operation::Select {
        comparisons: other_comparisons,
        ..
    } = &mut other_plan.steps[0].operation
    else {
        panic!("the test plan uses a select");
    };
    other_comparisons[0].origin = Some(ComparisonOriginId(0));

    assert_eq!(
        refusal(&plan, &KindVocabularies(Vec::new())).unwrap().1,
        Some(origin)
    );
    assert_eq!(
        refusal(&other_plan, &KindVocabularies(Vec::new()))
            .unwrap()
            .1,
        Some(other_origin)
    );
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

fn message(input: Input, relation: Relation, site: KindSite) -> Value {
    let (test, right, bound) = input();
    let plan = plan_comparing_kind(test, right, relation, site);
    let vocabularies = vocabularies();
    let message = first_unknown(&[plan], &[bound], &vocabularies)
        .map(|unknown| unknown_message(&unknown.word, &vocabularies).0)
        .unwrap_or_default();
    Box::leak(message)
}

fn subject(input: Input) -> Value {
    let expected = message(input, Relation::SyntaxTree, KindSite::Select);
    for relation in [Relation::SyntaxTree, Relation::SemanticSyntax] {
        for site in [KindSite::Select, KindSite::Join] {
            assert_eq!(
                message(input, relation, site),
                expected,
                "kind validation must agree for native and semantic selections and right joins"
            );
        }
    }
    expected
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
