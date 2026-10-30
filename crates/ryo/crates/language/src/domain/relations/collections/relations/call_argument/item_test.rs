//! The `call_argument` relation's cases.

use super::super::interface::RelationsItem as _;
use super::item::{CallArgument, CallArgumentRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Integer, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;

/// A row to write: every column holds a number of its own, so that a column moved is a column found.
type Input = ();
/// The numbers the row is stored as, in column order.
type Value = [Bits; 6];
type Failure = ();

fn row() -> CallArgumentRow {
    CallArgumentRow {
        call: NodeId(3),
        position: Integer(5),
        expression: NodeId(7),
        passes: Symbol(11),
        file: FileId(13),
        span: Span {
            start: Offset(17),
            end: Offset(19),
        },
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
        Bits(13),
        Bits(73_014_444_051),
    ]),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    Ok(CallArgument::words(row()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
