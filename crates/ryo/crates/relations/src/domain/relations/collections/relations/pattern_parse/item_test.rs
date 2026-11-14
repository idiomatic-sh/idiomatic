//! Typed item contract for pattern parse outcomes.
use super::super::interface::RelationsItem as _;
use super::item::{PatternParse, PatternParseRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;
type Input = ();
type Value = [Bits; 6];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("pattern_parse_preserves_typed_columns"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([
        Bits(1),
        Bits(2),
        Bits(3),
        Bits(4),
        Bits(5),
        Bits((0x6_u64 << 32_u32) | 0x7),
    ]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(PatternParse::words(PatternParseRow {
        carrier: NodeId(1),
        flags: Symbol(2),
        outcome: Symbol(3),
        error: Symbol(4),
        file: FileId(5),
        span: Span {
            start: Offset(6),
            end: Offset(7),
        },
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
