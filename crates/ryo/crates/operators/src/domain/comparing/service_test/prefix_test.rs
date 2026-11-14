//! Where the comparisons no row can fail end: the fixed prefix the keyed-select rewrite and the engine's root kinds stop at.
use super::column;
use crate::domain::comparing::service::fixed_prefix;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ComparisonOrigin, LiteralText, Operand, ParameterId, PlanComparison, QueryLiteral, Test,
};

/// What a row of the cases is: a node, then three names.
const KINDS: &[CellKind] = &[
    CellKind::Node,
    CellKind::Symbol,
    CellKind::Symbol,
    CellKind::Symbol,
];

/// The comparison of `left` and `right` by `test`.
fn compared(test: Test, left: Operand, right: Operand) -> PlanComparison {
    PlanComparison {
        test,
        left,
        right,
        origin: ComparisonOrigin::Unrecorded,
    }
}

/// Column `at` against the written text `text`.
fn written(test: Test, at: u32, text: &str) -> PlanComparison {
    compared(
        test,
        Operand::Column(column(at)),
        Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
    )
}

/// How many comparisons the fixed prefix of `comparisons` holds.
type Input = fn() -> Vec<PlanComparison>;
type Value = usize;
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_kind_test_and_an_equality_to_a_text_are_both_fixed"),
        word: Word::Succeeds,
        input: || {
            || {
                vec![
                    written(Test::Equal, 1, "call"),
                    written(Test::Equal, 3, "n1"),
                ]
            }
        },
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_written_text_on_the_left_is_fixed_too"),
        word: Word::Succeeds,
        input: || {
            || {
                vec![compared(
                    Test::NotEqual,
                    Operand::Literal(QueryLiteral::Text(LiteralText("n1".into()))),
                    Operand::Column(column(3)),
                )]
            }
        },
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_affix_of_a_written_text_is_fixed"),
        word: Word::Succeeds,
        input: || {
            || {
                vec![
                    written(Test::StartsWith, 3, "n"),
                    written(Test::EndsWith, 2, "a"),
                ]
            }
        },
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_pattern_read_from_the_row_ends_the_prefix_before_a_fixed_key"),
        word: Word::Succeeds,
        input: || {
            || {
                vec![
                    compared(
                        Test::Matches,
                        Operand::Column(column(2)),
                        Operand::Column(column(2)),
                    ),
                    written(Test::Equal, 3, "n1"),
                ]
            }
        },
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_column_past_the_row_ends_the_prefix"),
        word: Word::Succeeds,
        input: || || vec![written(Test::Equal, 3, "n1"), written(Test::Equal, 4, "n2")],
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_parameter_ends_the_prefix"),
        word: Word::Succeeds,
        input: || {
            || {
                vec![compared(
                    Test::Equal,
                    Operand::Column(column(3)),
                    Operand::Parameter(ParameterId(0)),
                )]
            }
        },
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_written_pattern_is_not_fixed"),
        word: Word::Succeeds,
        input: || || vec![written(Test::Matches, 3, "n.*")],
        expect: Expect::Value(0),
    },
];

fn subject(comparisons: Input) -> Result<Value, Failure> {
    let comparisons = comparisons();
    Ok(fixed_prefix(&comparisons, KINDS).0.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
