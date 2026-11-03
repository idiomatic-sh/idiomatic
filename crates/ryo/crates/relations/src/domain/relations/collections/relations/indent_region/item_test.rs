//! Distinct stored words prove the shared catalog column layout.
use super::super::interface::RelationsItem as _;
use super::item::{IndentRegion, IndentRegionRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    relations::types::{Bits, FileId, Integer, NodeId, Offset, Span},
    terms::types::Symbol,
};
type Input = ();
type Value = [Bits; 7];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("region columns preserve identities and byte coordinates"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([
        Bits(3),
        Bits(5),
        Bits(7),
        Bits(11),
        Bits(13),
        Bits(17),
        Bits(47_244_640_269),
    ]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(IndentRegion::words(IndentRegionRow {
        id: NodeId(3),
        parent: NodeId(5),
        indentation: Symbol(7),
        start_byte: Integer(11),
        end_byte: Integer(13),
        file: FileId(17),
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
