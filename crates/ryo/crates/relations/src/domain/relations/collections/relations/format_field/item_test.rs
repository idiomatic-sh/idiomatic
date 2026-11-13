//! The field shape distinguishes positional conversion selectors from decoded mapping keys.
use super::super::interface::RelationsItem as _;
use super::item::{FormatField, FormatFieldRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Integer, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;

type Input = ();
type Value = [Bits; 9];
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("format field retains its conversion and consumption modes"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([
        Bits(1),
        Bits(2),
        Bits(3),
        Bits(4),
        Bits(5),
        Bits(6),
        Bits(7),
        Bits(8),
        Bits((0x9_u64 << 32_u32) | 0x9),
    ]),
}];

fn subject(_: Input) -> Result<Value, Failure> {
    Ok(FormatField::words(FormatFieldRow {
        id: NodeId(1),
        ordinal: Integer(2),
        argument_kind: Symbol(3),
        mapping_key: Symbol(4),
        conversion: Symbol(5),
        width_mode: Symbol(6),
        precision_mode: Symbol(7),
        file: FileId(8),
        span: Span {
            start: Offset(9),
            end: Offset(9),
        },
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
