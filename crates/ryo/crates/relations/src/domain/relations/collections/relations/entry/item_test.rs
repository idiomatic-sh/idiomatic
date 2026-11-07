//! The `entry` relation's cases.

use super::super::interface::RelationsItem as _;
use super::item::{Entry, EntryRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Flag, Integer, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;
use crate::domain::value_kind::types::ValueKind;

/// A row to write: every column holds a number of its own, so that a column moved is a column found.
type Input = ();
/// The numbers the row is stored as, in column order.
type Value = [Bits; 14];
type Failure = ();

fn row() -> EntryRow {
    EntryRow {
        decoded: Symbol(3),
        key: Symbol(5),
        kind: ValueKind::Boolean,
        occurrence: NodeId(11),
        ordinal: Integer(13),
        owner: NodeId(17),
        file: FileId(19),
        span: Span {
            start: Offset(23),
            end: Offset(27),
        },
        text: Symbol(29),
        boolean: Flag::Raised,
        integer: Integer(-31),
        float: Symbol(37),
        datetime: Symbol(41),
        radix: Integer(16),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_row_is_14_numbers_in_column_order"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([
        Bits(3),
        Bits(5),
        Bits(4_294_902_275),
        Bits(11),
        Bits(13),
        Bits(17),
        Bits(19),
        Bits(98_784_247_835),
        Bits(29),
        Bits(1),
        Bits(18_446_744_073_709_551_585),
        Bits(37),
        Bits(41),
        Bits(16),
    ]),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    Ok(Entry::words(row()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
