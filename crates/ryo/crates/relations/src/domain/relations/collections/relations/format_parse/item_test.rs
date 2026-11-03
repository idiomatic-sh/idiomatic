//! The parser outcome, not the summary's zero values, distinguishes empty, malformed and dynamic inputs.
use super::super::interface::RelationsItem as _;
use super::item::{FormatParse, FormatParseRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Flag, Integer, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;

type Input = ();
type Value = [Bits; 13];
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("format parser output keeps error state and summary columns distinct"),
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
        Bits(1),
        Bits(0),
        Bits(1),
        Bits(12),
        Bits((0xd_u64 << 32_u32) | 0xd),
    ]),
}];

fn subject(_: Input) -> Result<Value, Failure> {
    Ok(FormatParse::words(FormatParseRow {
        id: NodeId(1),
        dialect: Symbol(2),
        representation: Symbol(3),
        outcome: Symbol(4),
        error_category: Symbol(5),
        error_character: Symbol(6),
        error_index: Integer(7),
        positional_consumption: Integer(8),
        mapping_present: Flag::Raised,
        dynamic_width: Flag::Lowered,
        dynamic_precision: Flag::Raised,
        file: FileId(12),
        span: Span {
            start: Offset(13),
            end: Offset(13),
        },
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
