//! Typed item contract for brace-format outcomes.
use super::super::interface::RelationsItem;
use super::item::{BraceFormatParse, BraceFormatParseRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Integer, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;
type Input = ();
type Value = [Bits; 10];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("brace_format_parse_preserves_typed_columns"),
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
        Bits((10_u64 << 32) | 10),
    ]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(BraceFormatParse::words(BraceFormatParseRow {
        id: NodeId(1),
        representation: Symbol(2),
        outcome: Symbol(3),
        error_category: Symbol(4),
        automatic_count: Integer(5),
        indexed_count: Integer(6),
        keyword_count: Integer(7),
        nested_field_count: Integer(8),
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
