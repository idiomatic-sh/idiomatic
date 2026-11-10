//! Decoded fragments keep their owning string, text, file and source interval in typed columns.

use super::super::interface::RelationsItem as _;
use super::item::{InterpolatedStringLiteral, InterpolatedStringLiteralRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;

type Input = ();
type Value = [Bits; 5];
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_fragment_keeps_its_owner_text_file_and_span"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(5), Bits(7), Bits(11), Bits(55_834_574_861)]),
}];

fn subject(_: Input) -> Result<Value, Failure> {
    Ok(InterpolatedStringLiteral::words(
        InterpolatedStringLiteralRow {
            id: NodeId(3),
            owner: NodeId(5),
            value: Symbol(7),
            file: FileId(11),
            span: Span {
                start: Offset(13),
                end: Offset(13),
            },
        },
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
