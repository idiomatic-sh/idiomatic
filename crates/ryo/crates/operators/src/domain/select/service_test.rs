//! The `select` service's cases, on values alone.

use super::service::lower;
use super::types::SelectParts;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// A scenario: whether what the part answers is what the case states.
type Input = fn() -> bool;
type Value = bool;
type Failure = ();

/// A select with no comparison keeps the columns it names, in the order it names them, repeats kept.
fn keeps_its_columns_in_order() -> bool {
    lower(&[], &[ColumnIndex(2), ColumnIndex(0), ColumnIndex(2)], &[])
        == SelectParts {
            comparisons: Vec::new(),
            kept: vec![ColumnIndex(2), ColumnIndex(0), ColumnIndex(2)],
            bindings: Vec::new(),
        }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_select_keeps_its_columns_in_the_order_named"),
    word: Word::Succeeds,
    input: || keeps_its_columns_in_order,
    expect: Expect::Value(true),
}];

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(input())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
