//! The `semantic_child` relation's cases.

use super::super::interface::RelationsItem as _;
use super::item::{SemanticChild, SemanticChildRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Integer, NodeId};
use crate::domain::terms::types::Symbol;

/// A row to write: every column holds a number of its own, so that a column moved is a column found.
type Input = ();
/// The numbers the row is stored as, in column order.
type Value = [Bits; 5];
type Failure = ();

fn row() -> SemanticChildRow {
    SemanticChildRow {
        parent: NodeId(3),
        child: NodeId(5),
        ordinal: Integer(7),
        field: Symbol(11),
        file: FileId(13),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_row_is_5_numbers_in_column_order"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(5), Bits(7), Bits(11), Bits(13)]),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    Ok(SemanticChild::words(row()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
