//! The `parameter` relation's cases.

use super::super::interface::RelationsItem as _;
use super::item::{Parameter, ParameterRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::parameter_kind::types::ParameterKind;
use crate::domain::relations::types::{Bits, FileId, Integer, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;

/// A row to write: every column holds a number of its own, so that a column moved is a column found.
type Input = ();
/// The numbers the row is stored as, in column order.
type Value = [Bits; 8];
type Failure = ();

fn row() -> ParameterRow {
    ParameterRow {
        id: NodeId(3),
        kind: ParameterKind::PositionalOrKeyword,
        refinement: Symbol(7),
        name: Symbol(11),
        ordinal: Integer(13),
        owner: NodeId(17),
        file: FileId(19),
        span: Span {
            start: Offset(23),
            end: Offset(27),
        },
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_row_is_8_numbers_in_column_order"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([
        Bits(3),
        Bits(4_294_902_018),
        Bits(7),
        Bits(11),
        Bits(13),
        Bits(17),
        Bits(19),
        Bits(98_784_247_835),
    ]),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    Ok(Parameter::words(row()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
