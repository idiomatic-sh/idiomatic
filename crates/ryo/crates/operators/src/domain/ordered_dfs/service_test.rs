//! The `ordered_dfs` service's cases, on values alone.

use super::service::lower;
use super::types::{DfsParts, DfsShape};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// A scenario: whether what the part answers is what the case states.
type Input = fn() -> bool;
type Value = bool;
type Failure = ();

/// The edge columns are read as source, destination, order and edge, in that order, and each side keeps its keys.
fn reads_its_edge_columns_in_order() -> bool {
    lower(DfsShape {
        root: ColumnIndex(0),
        edge_columns: [
            ColumnIndex(3),
            ColumnIndex(2),
            ColumnIndex(1),
            ColumnIndex(0),
        ],
        root_keys: &[ColumnIndex(1)],
        edge_keys: &[ColumnIndex(4)],
    }) == DfsParts {
        root: ColumnIndex(0),
        source: ColumnIndex(3),
        destination: ColumnIndex(2),
        order: ColumnIndex(1),
        edge: ColumnIndex(0),
        root_keys: vec![ColumnIndex(1)],
        edge_keys: vec![ColumnIndex(4)],
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("an_ordered_walk_reads_its_edge_columns_in_order"),
    word: Word::Succeeds,
    input: || reads_its_edge_columns_in_order,
    expect: Expect::Value(true),
}];

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(input())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
