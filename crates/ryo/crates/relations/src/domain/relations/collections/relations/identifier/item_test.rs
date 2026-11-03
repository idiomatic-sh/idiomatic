//! Distinct values guard the inherited identifier row layout.
use super::super::interface::RelationsItem as _;
use super::item::{Identifier, IdentifierRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    relations::types::{Bits, FileId, NodeId, Offset, Span},
    terms::types::Symbol,
};
type Input = ();
type Value = [Bits; 4];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("canonical identifier keeps original byte span"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(5), Bits(7), Bits(47_244_640_269)]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(Identifier::words(IdentifierRow {
        id: NodeId(3),
        name: Symbol(5),
        file: FileId(7),
        span: Span {
            start: Offset(11),
            end: Offset(13),
        },
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
