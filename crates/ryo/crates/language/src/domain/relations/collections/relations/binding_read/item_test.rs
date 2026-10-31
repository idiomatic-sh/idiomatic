//! Independent distinct cells guard the public binding fact layout.
use super::super::interface::RelationsItem as _;
use super::item::{BindingRead, BindingReadRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, NodeId, Offset, Span};
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
        Bits(47244640269),
    ]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(BindingRead::words(BindingReadRow {
        reference: NodeId(3),
        name: Symbol(5),
        scope: NodeId(7),
        phase: NodeId(9),
        context: Symbol(11),
        state: Symbol(13),
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
