//! The `worklist` service's cases, on values alone.

use super::service::lower;
use super::types::{WorklistParts, WorklistShape};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// A scenario: whether what the part answers is what the case states.
type Input = fn() -> bool;
type Value = bool;
type Failure = ();

/// The edge columns are read as source, destination, order, before-state and after-state, the lattice columns as lower and
/// upper, in that order, and each side keeps its keys.
fn reads_its_columns_in_order() -> bool {
    lower(WorklistShape {
        root: ColumnIndex(0),
        state: ColumnIndex(1),
        edge_columns: [
            ColumnIndex(6),
            ColumnIndex(5),
            ColumnIndex(4),
            ColumnIndex(3),
            ColumnIndex(2),
        ],
        lattice_columns: [ColumnIndex(8), ColumnIndex(7)],
        root_keys: &[ColumnIndex(9)],
        edge_keys: &[ColumnIndex(10)],
    }) == WorklistParts {
        root: ColumnIndex(0),
        state: ColumnIndex(1),
        source: ColumnIndex(6),
        destination: ColumnIndex(5),
        order: ColumnIndex(4),
        before: ColumnIndex(3),
        after: ColumnIndex(2),
        lower: ColumnIndex(8),
        upper: ColumnIndex(7),
        root_keys: vec![ColumnIndex(9)],
        edge_keys: vec![ColumnIndex(10)],
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_worklist_reads_its_columns_in_order"),
    word: Word::Succeeds,
    input: || reads_its_columns_in_order,
    expect: Expect::Value(true),
}];

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(input())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
