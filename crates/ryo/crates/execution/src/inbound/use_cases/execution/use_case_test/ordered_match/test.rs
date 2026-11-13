//! Original independent queue cases, compiled and published by the public owner.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{
            ColumnIndex, HostedNumber, Input as PlanInput, JoinColumn, Side, StepId,
        },
        plans::types::Operation,
    },
};
use std::borrow::Cow;
#[derive(Clone, Copy, Debug)]
struct Scenario {
    left: &'static [(i64, i64)],
    right: &'static [(i64, i64)],
}
/// What the match published: its row count, its numeric (opened, closed) pairs in order, and any row that is not two numbers.
#[derive(Debug, PartialEq, Eq)]
struct Matched {
    rows: usize,
    pairs: Cow<'static, [(i64, i64)]>,
    other_rows: usize,
}
type Input = Scenario;
type Value = Matched;
type Failure = OperatingError;
fn subject(input: Input) -> Result<Value, Failure> {
    let left: Vec<_> = input
        .left
        .iter()
        .map(|(key, order)| [*key, *order])
        .collect();
    let right: Vec<_> = input
        .right
        .iter()
        .map(|(key, order)| [*key, *order])
        .collect();
    let hosted = vec![
        fixtures::requests::numbers(
            "left",
            2,
            &left.iter().map(|row| row.as_slice()).collect::<Vec<_>>(),
        ),
        fixtures::requests::numbers(
            "right",
            2,
            &right.iter().map(|row| row.as_slice()).collect::<Vec<_>>(),
        ),
    ];
    let plan = inputs::plan(vec![
        Operation::Hosted {
            relation: HostedNumber(0),
        },
        Operation::Hosted {
            relation: HostedNumber(1),
        },
        Operation::OrderedMatch {
            left: PlanInput::Step(StepId(0)),
            right: PlanInput::Step(StepId(1)),
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
        },
    ]);
    let output = fixtures::requests::published(plan, vec![], hosted, &mut Symbols::default())?;
    let rows = output.cells.chunks_exact(2);
    let pairs: Vec<_> = rows
        .clone()
        .filter_map(|row| match row {
            [Cell::Number(opened), Cell::Number(closed)] => Some((opened.0, closed.0)),
            _ => None,
        })
        .collect();
    Ok(Matched {
        rows: output.count.0,
        other_rows: rows.len() - pairs.len(),
        pairs: pairs.into(),
    })
}
const fn matched(pairs: &'static [(i64, i64)]) -> Expect<Value, Failure> {
    Expect::Value(Matched {
        rows: pairs.len(),
        pairs: Cow::Borrowed(pairs),
        other_rows: 0,
    })
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("signed_self_match_pairs_every_nonmaximum_with_its_successor"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(2, 0), (1, -1), (1, -9), (2, -7), (1, -3)],
            right: &[(2, 0), (1, -1), (1, -9), (2, -7), (1, -3)],
        },
        expect: matched(&[(-9, -3), (-3, -1), (-7, 0)]),
    },
    Case {
        name: CaseName("adjacent_large_integer_orders_remain_distinct"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, 9_007_199_254_740_992), (1, 9_007_199_254_740_993)],
            right: &[(1, 9_007_199_254_740_993), (1, 9_007_199_254_740_994)],
        },
        expect: matched(&[
            (9_007_199_254_740_992, 9_007_199_254_740_993),
            (9_007_199_254_740_993, 9_007_199_254_740_994),
        ]),
    },
    Case {
        name: CaseName("full_signed_integer_order_domain"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, i64::MIN)],
            right: &[(1, i64::MAX)],
        },
        expect: matched(&[(i64::MIN, i64::MAX)]),
    },
    Case {
        name: CaseName("two_requests_two_replies"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, 2), (1, 3)],
            right: &[(1, 4), (1, 5)],
        },
        expect: matched(&[(2, 4), (3, 5)]),
    },
    Case {
        name: CaseName("reply_before_request_is_unmatched"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, 2)],
            right: &[(1, 1), (1, 3)],
        },
        expect: matched(&[(2, 3)]),
    },
    Case {
        name: CaseName("extra_reply_does_not_consume_future_request"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, 2), (1, 6)],
            right: &[(1, 3), (1, 4), (1, 7)],
        },
        expect: matched(&[(2, 3), (6, 7)]),
    },
    Case {
        name: CaseName("input_permutation_has_no_effect"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, 3), (1, 2)],
            right: &[(1, 5), (1, 4)],
        },
        expect: matched(&[(2, 4), (3, 5)]),
    },
    Case {
        name: CaseName("key_groups_are_independent"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(2, 1), (1, 2)],
            right: &[(1, 3), (2, 4)],
        },
        expect: matched(&[(2, 3), (1, 4)]),
    },
    Case {
        name: CaseName("equal_orders_do_not_match"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, 2)],
            right: &[(1, 2), (1, 3)],
        },
        expect: matched(&[(2, 3)]),
    },
    Case {
        name: CaseName("unmatched_request_has_no_row"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, 9)],
            right: &[(1, 2)],
        },
        expect: matched(&[]),
    },
    Case {
        name: CaseName("empty_inputs"),
        word: Word::Succeeds,
        input: || Input {
            left: &[],
            right: &[],
        },
        expect: matched(&[]),
    },
    Case {
        name: CaseName("duplicate_left_order_refused"),
        word: Word::Refuses,
        input: || Input {
            left: &[(1, 2), (1, 2)],
            right: &[(1, 3), (1, 4)],
        },
        expect: Expect::Error(OperatingError::DuplicateOrder),
    },
    Case {
        name: CaseName("duplicate_unmatched_right_group_refused"),
        word: Word::Refuses,
        input: || Input {
            left: &[],
            right: &[(2, 3), (2, 3)],
        },
        expect: Expect::Error(OperatingError::DuplicateOrder),
    },
    Case {
        name: CaseName("same_order_in_distinct_groups_valid"),
        word: Word::Succeeds,
        input: || Input {
            left: &[(1, 2), (2, 2)],
            right: &[(1, 3), (2, 3)],
        },
        expect: matched(&[(2, 3), (2, 3)]),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
