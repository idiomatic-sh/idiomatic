//! The `cycles` service's cases, on values alone.

use super::service::lower;
use super::types::CycleParts;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// A scenario: whether what the part answers is what the case states.
type Input = fn() -> bool;
type Value = bool;
type Failure = ();

/// A cycle search runs over the edges from its source column to its destination column.
fn searches_its_edges() -> bool {
    lower(ColumnIndex(1), ColumnIndex(0))
        == CycleParts {
            source: ColumnIndex(1),
            destination: ColumnIndex(0),
        }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_cycle_search_runs_over_its_edge_columns"),
    word: Word::Succeeds,
    input: || searches_its_edges,
    expect: Expect::Value(true),
}];

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(input())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
