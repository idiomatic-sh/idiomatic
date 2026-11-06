//! Global order and local conversion identity remain separate typed columns.
use super::super::interface::RelationsItem as _;
use super::item::{RawFormatField, RawFormatFieldRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Integer, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;
type Input = ();
type Value = [Bits; 13];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("raw_field_preserves_both_ordinals"),
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
        Bits(9),
        Bits(10),
        Bits(11),
        Bits((0xc_u64 << 32_u32) | 0xd),
        Bits(14)
    ]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(RawFormatField::words(RawFormatFieldRow {
        id: NodeId(1),
        segment: NodeId(2),
        segment_ordinal: Integer(3),
        ordinal: Integer(4),
        segment_field_ordinal: Integer(5),
        argument_kind: Symbol(6),
        mapping_key: Symbol(7),
        conversion: Symbol(8),
        width_mode: Symbol(9),
        precision_mode: Symbol(10),
        file: FileId(11),
        flags: Symbol(14),
        span: Span {
            start: Offset(12),
            end: Offset(13),
        },
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
