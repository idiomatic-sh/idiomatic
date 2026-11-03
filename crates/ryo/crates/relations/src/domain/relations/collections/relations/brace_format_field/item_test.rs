//! Typed item contract for brace-format fields.
use super::super::interface::RelationsItem as _;
use super::item::{BraceFormatField, BraceFormatFieldRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Flag, Integer, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;
type Input = ();
type Value = [Bits; 10];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("brace_format_field_preserves_typed_columns"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([
        Bits(1),
        Bits(2),
        Bits(3),
        Bits(4),
        Bits(5),
        Bits(6),
        Bits(1),
        Bits(8),
        Bits(9),
        Bits((0xa_u64 << 32_u32) | 0xa),
    ]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(BraceFormatField::words(BraceFormatFieldRow {
        id: NodeId(1),
        ordinal: Integer(2),
        context: Symbol(3),
        parent_ordinal: Integer(4),
        selector_kind: Symbol(5),
        selector_index: Integer(6),
        index_overflow: Flag::Raised,
        selector_name: Symbol(8),
        file: FileId(9),
        span: Span {
            start: Offset(10),
            end: Offset(10),
        },
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
