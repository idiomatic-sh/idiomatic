//! Independent queue scenarios for ordered matching.

use crate::application::file_dag::testing::TestOperator as _;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::ordered_match::item::OrderedMatchItem;
use crate::domain::table_model::types::Rows;
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Relation, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run, selects};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput, JoinColumn, Side};
use ryo_values::domain::plans::types::{Operation, QueryOperator};
use ryo_values::domain::rows::service::push_row;

#[derive(Clone, Copy, Debug)]
struct Scenario {
    left: &'static [(i64, i64)],
    right: &'static [(i64, i64)],
    expected: &'static [(i64, i64)],
}
type Input = Scenario;
type Value = bool;
type Failure = OperatingError;

fn table(rows: &[(i64, i64)]) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::Number; 2],
        ..Table::default()
    };
    for (key, order) in rows {
        push_row(
            &mut table,
            &[Cell::Number(Integer(*key)), Cell::Number(Integer(*order))],
        );
    }
    table
}

fn subject(input: Input) -> Result<Value, Failure> {
    let left = table(input.left);
    let right = table(input.right);
    let operation = Operation::OrderedMatch {
        left: PlanInput::Relation(Relation::Function),
        right: PlanInput::Relation(Relation::Function),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        left_order: ColumnIndex(1),
        right_order: ColumnIndex(1),
        output: vec![
            JoinColumn {
                side: Side::Left,
                column: ColumnIndex(1),
            },
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(1),
            },
        ],
    };
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut scratch,
    };
    let mut output = Table::default();
    OrderedMatchItem.compute(
        &operation,
        &[Rows::Computed(&left), Rows::Computed(&right)],
        &mut context,
        &mut output,
    )?;
    let actual: Vec<_> = output
        .cells
        .chunks_exact(2)
        .map(|row| match row {
            [Cell::Number(opened), Cell::Number(closed)] => (opened.0, closed.0),
            _ => panic!("numeric output required"),
        })
        .collect();
    assert_eq!(actual, input.expected);
    Ok(true)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("signed_self_match_pairs_every_nonmaximum_with_its_successor"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(2, 0), (1, -1), (1, -9), (2, -7), (1, -3)],
            right: &[(2, 0), (1, -1), (1, -9), (2, -7), (1, -3)],
            expected: &[(-9, -3), (-3, -1), (-7, 0)],
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("adjacent_large_integer_orders_remain_distinct"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, 9_007_199_254_740_992), (1, 9_007_199_254_740_993)],
            right: &[(1, 9_007_199_254_740_993), (1, 9_007_199_254_740_994)],
            expected: &[
                (9_007_199_254_740_992, 9_007_199_254_740_993),
                (9_007_199_254_740_993, 9_007_199_254_740_994),
            ],
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("full_signed_integer_order_domain"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, i64::MIN)],
            right: &[(1, i64::MAX)],
            expected: &[(i64::MIN, i64::MAX)],
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("two_requests_two_replies"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, 2), (1, 3)],
            right: &[(1, 4), (1, 5)],
            expected: &[(2, 4), (3, 5)],
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("reply_before_request_is_unmatched"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, 2)],
            right: &[(1, 1), (1, 3)],
            expected: &[(2, 3)],
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("extra_reply_does_not_consume_future_request"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, 2), (1, 6)],
            right: &[(1, 3), (1, 4), (1, 7)],
            expected: &[(2, 3), (6, 7)],
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("input_permutation_has_no_effect"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, 3), (1, 2)],
            right: &[(1, 5), (1, 4)],
            expected: &[(2, 4), (3, 5)],
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("key_groups_are_independent"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(2, 1), (1, 2)],
            right: &[(1, 3), (2, 4)],
            expected: &[(2, 3), (1, 4)],
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("equal_orders_do_not_match"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, 2)],
            right: &[(1, 2), (1, 3)],
            expected: &[(2, 3)],
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("unmatched_request_has_no_row"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, 9)],
            right: &[(1, 2)],
            expected: &[],
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("empty_inputs"),
        word: Word::Succeeds,
        input: || Input {
            left: &[],
            right: &[],
            expected: &[],
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("duplicate_left_order_refused"),
        word: Word::Refuses,
        input: || Input {
            left: &[(1, 2), (1, 2)],
            right: &[(1, 3), (1, 4)],
            expected: &[],
        },
        expect: Expect::Refusal(|error| selects(matches!(error, OperatingError::DuplicateOrder))),
    },
    Case {
        name: CaseName("duplicate_unmatched_right_group_refused"),
        word: Word::Refuses,
        input: || Input {
            left: &[],
            right: &[(2, 3), (2, 3)],
            expected: &[],
        },
        expect: Expect::Refusal(|error| selects(matches!(error, OperatingError::DuplicateOrder))),
    },
    Case {
        name: CaseName("same_order_in_distinct_groups_valid"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, 2), (2, 2)],
            right: &[(1, 3), (2, 3)],
            expected: &[(2, 3), (2, 3)],
        },
        expect: Expect::Value(true),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    crate::domain::operator::collections::operators::fixtures::contract::check(
        QueryOperator::OrderedMatch,
    );
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::rows::types::Table;
