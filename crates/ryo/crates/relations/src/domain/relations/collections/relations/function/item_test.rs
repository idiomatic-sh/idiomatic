//! The `function` relation's cases.

use super::super::interface::RelationsItem as _;
use super::item::{Function, FunctionRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;

/// A row to write: every column holds a number of its own, so that a column moved is a column found.
type Input = ();
/// The numbers the row is stored as, in column order.
type Value = [Bits; 4];
type Failure = ();

fn row() -> FunctionRow {
    FunctionRow {
        id: NodeId(3),
        name: Symbol(5),
        file: FileId(7),
        span: Span {
            start: Offset(11),
            end: Offset(15),
        },
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_row_is_4_numbers_in_column_order"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(5), Bits(7), Bits(47_244_640_271)]),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    Ok(Function::words(row()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
