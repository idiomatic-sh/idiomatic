//! The `sequence_element` relation's cases.

use super::super::interface::RelationsItem as _;
use super::item::{SequenceElement, SequenceElementRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Flag, Integer, NodeId, Offset, Span};
use crate::domain::terms::types::{Symbol, ValueKind};

/// A row to write: every column holds a number of its own, so that a column moved is a column found.
type Input = ();
/// The numbers the row is stored as, in column order.
type Value = [Bits; 13];
type Failure = ();

fn row() -> SequenceElementRow {
    SequenceElementRow {
        decoded: Symbol(3),
        kind: ValueKind::Boolean,
        occurrence: NodeId(7),
        ordinal: Integer(11),
        owner: NodeId(13),
        file: FileId(17),
        span: Span {
            start: Offset(19),
            end: Offset(23),
        },
        text: Symbol(23),
        boolean: Flag::Raised,
        integer: Integer(-31),
        float: Symbol(37),
        datetime: Symbol(41),
        radix: Integer(16),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_row_is_13_numbers_in_column_order"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([
        Bits(3),
        Bits(4_294_902_275),
        Bits(7),
        Bits(11),
        Bits(13),
        Bits(17),
        Bits(81_604_378_647),
        Bits(23),
        Bits(1),
        Bits(18_446_744_073_709_551_585),
        Bits(37),
        Bits(41),
        Bits(16),
    ]),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    Ok(SequenceElement::words(row()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
