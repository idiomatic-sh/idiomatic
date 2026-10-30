//! The `source_dependency` relation's cases.

use super::super::interface::RelationsItem as _;
use super::item::{SourceDependency, SourceDependencyRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Offset, Span};
use crate::domain::terms::types::Symbol;

/// A row to write: every column holds a number of its own, so that a column moved is a column found.
type Input = ();
/// The numbers the row is stored as, in column order.
type Value = [Bits; 4];
type Failure = ();

fn row() -> SourceDependencyRow {
    SourceDependencyRow {
        source: FileId(3),
        target: FileId(5),
        span: Span {
            start: Offset(7),
            end: Offset(11),
        },
        written: Symbol(13),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_row_is_4_numbers_in_column_order"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(5), Bits(30_064_771_083), Bits(13)]),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    Ok(SourceDependency::words(row()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
