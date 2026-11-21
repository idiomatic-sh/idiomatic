//! The `ordered_match` service's cases, on values alone.

use super::service::{lower, paired};
use super::types::OrderedParts;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::ordered_groups::types::GroupOrders;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};

/// A scenario: whether what the part answers is what the case states.
type Input = fn() -> bool;
type Value = bool;
type Failure = ();

/// An ordered match keeps both sides' keys and orders and its output columns.
fn keeps_both_sides() -> bool {
    let output = [JoinColumn {
        side: Side::Left,
        column: ColumnIndex(1),
    }];
    lower(
        &[ColumnIndex(0)],
        &[ColumnIndex(2)],
        ColumnIndex(1),
        ColumnIndex(3),
        &output,
    ) == OrderedParts {
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(2)],
        left_order: ColumnIndex(1),
        right_order: ColumnIndex(3),
        output: output.to_vec(),
        band: None,
    }
}

/// The pairs of one key whose left rows have the orders `left` and whose right rows the orders `right`, as (left place,
/// right place).
fn pairs(left: &[i64], right: &[i64]) -> Vec<(u32, u32)> {
    let orders =
        |each: &[i64]| -> Vec<Integer> { each.iter().map(|order| Integer(*order)).collect() };
    let (left, right) = (orders(left), orders(right));
    let mut found = Vec::new();
    paired(GroupOrders(&left), GroupOrders(&right), &mut found);
    found
        .iter()
        .map(|pair| (pair.left.0, pair.right.0))
        .collect()
}

/// Each left row takes the earliest right row after it that no earlier left row took.
fn pairs_first_in_first_out() -> bool {
    pairs(&[1, 2, 3], &[1, 2, 3]) == [(0, 1), (1, 2)]
        && pairs(&[1, 5], &[2, 3, 9]) == [(0, 0), (1, 2)]
        // Two left rows before two right rows take one each: a right row is taken once.
        && pairs(&[1, 2], &[3, 4]) == [(0, 0), (1, 1)]
}

/// A right row of the same order is not after its left row.
fn an_equal_order_is_not_later() -> bool {
    pairs(&[2], &[2]).is_empty() && pairs(&[2], &[2, 3]) == [(0, 1)]
}

/// A left row with no later right row takes none, and a side with no row gives no pair.
fn a_left_row_with_no_later_right_row_takes_none() -> bool {
    pairs(&[1, 9], &[4]) == [(0, 0)] && pairs(&[], &[4]).is_empty() && pairs(&[1], &[]).is_empty()
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_ordered_match_keeps_both_sides"),
        word: Word::Succeeds,
        input: || keeps_both_sides,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_pairing_is_first_in_first_out"),
        word: Word::Succeeds,
        input: || pairs_first_in_first_out,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_equal_order_is_not_later"),
        word: Word::Succeeds,
        input: || an_equal_order_is_not_later,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_left_row_with_no_later_right_row_takes_none"),
        word: Word::Succeeds,
        input: || a_left_row_with_no_later_right_row_takes_none,
        expect: Expect::Value(true),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(input())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
