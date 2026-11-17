//! The `ordered_match` service's cases, on values alone.

use super::service::lower;
use super::types::OrderedParts;
use crate::cases::{Case, CaseName, Expect, Word, run};
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
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("an_ordered_match_keeps_both_sides"),
    word: Word::Succeeds,
    input: || keeps_both_sides,
    expect: Expect::Value(true),
}];

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(input())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
