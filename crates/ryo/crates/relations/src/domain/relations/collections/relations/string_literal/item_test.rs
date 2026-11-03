//! The `string_literal` relation's cases.

use super::super::interface::RelationsItem as _;
use super::item::{StringLiteral, StringLiteralRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Flag, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;

/// A row to write: every column holds a number of its own, so that a column moved is a column found.
type Input = ();
/// The numbers the row is stored as, in column order.
type Value = [Bits; 6];
type Failure = ();

fn row() -> StringLiteralRow {
    StringLiteralRow {
        occurrence: NodeId(3),
        owner: NodeId(5),
        value: Symbol(7),
        file: FileId(11),
        span: Span {
            start: Offset(13),
            end: Offset(17),
        },
        import: Flag::Raised,
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_row_is_6_numbers_in_column_order"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([
        Bits(3),
        Bits(5),
        Bits(7),
        Bits(11),
        Bits(55_834_574_865),
        Bits(1),
    ]),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    Ok(StringLiteral::words(row()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
