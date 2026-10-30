//! The `reference` relation's cases.

use super::super::interface::RelationsItem as _;
use super::item::{Reference, ReferenceRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;

/// A row to write: every column holds a number of its own, so that a column moved is a column found.
type Input = ();
/// The numbers the row is stored as, in column order.
type Value = [Bits; 5];
type Failure = ();

fn row() -> ReferenceRow {
    ReferenceRow {
        occurrence: NodeId(3),
        owner: NodeId(5),
        target: Symbol(7),
        file: FileId(11),
        span: Span {
            start: Offset(13),
            end: Offset(17),
        },
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_row_is_5_numbers_in_column_order"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(5), Bits(7), Bits(11), Bits(55_834_574_865)]),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    Ok(Reference::words(row()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
