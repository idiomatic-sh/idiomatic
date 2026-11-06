//! Raw segment identity and malformed-state columns survive typed row encoding.
use super::super::interface::RelationsItem as _;
use super::item::{RawFormatSegment, RawFormatSegmentRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Flag, Integer, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;
type Input = ();
type Value = [Bits; 14];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("raw_segment_preserves_identity_and_outcome"),
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
        Bits(1),
        Bits(0),
        Bits(1),
        Bits(13),
        Bits((0xe_u64 << 32_u32) | 0xf)
    ]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(RawFormatSegment::words(RawFormatSegmentRow {
        id: NodeId(1),
        segment: NodeId(2),
        segment_ordinal: Integer(3),
        dialect: Symbol(4),
        outcome: Symbol(5),
        error_category: Symbol(6),
        error_character: Symbol(7),
        error_index: Integer(8),
        positional_consumption: Integer(9),
        mapping_present: Flag::Raised,
        dynamic_width: Flag::Lowered,
        dynamic_precision: Flag::Raised,
        file: FileId(13),
        span: Span {
            start: Offset(14),
            end: Offset(15),
        },
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
