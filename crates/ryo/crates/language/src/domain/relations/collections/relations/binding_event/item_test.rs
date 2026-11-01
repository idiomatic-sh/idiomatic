//! Independent distinct cells guard the public binding fact layout.
use super::super::interface::RelationsItem as _;
use super::item::{BindingEvent, BindingEventRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Integer, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;
type Input = ();
type Value = [Bits; 8];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("distinct identities and state preserve their columns"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([
        Bits(3),
        Bits(5),
        Bits(7),
        Bits(9),
        Bits(11),
        Bits(13),
        Bits(15),
        Bits(47_244_640_269),
    ]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(BindingEvent::words(BindingEventRow {
        occurrence: NodeId(3),
        name: Symbol(5),
        scope: NodeId(7),
        phase: NodeId(9),
        kind: Symbol(11),
        ordinal: Integer(13),
        file: FileId(15),
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
