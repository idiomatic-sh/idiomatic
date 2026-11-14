//! Typed item contract for pattern elements.
use super::super::interface::RelationsItem as _;
use super::item::{PatternSyntax, PatternSyntaxRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Integer, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;
type Input = ();
type Value = [Bits; 7];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("pattern_syntax_preserves_typed_columns"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([
        Bits(1),
        Bits(2),
        Bits(3),
        Bits(4),
        Bits(128_077),
        Bits(6),
        Bits((0x7_u64 << 32_u32) | 0x8),
    ]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(PatternSyntax::words(PatternSyntaxRow {
        id: NodeId(1),
        carrier: NodeId(2),
        kind: Symbol(3),
        text: Symbol(4),
        value: Integer(128_077),
        file: FileId(6),
        span: Span {
            start: Offset(7),
            end: Offset(8),
        },
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
