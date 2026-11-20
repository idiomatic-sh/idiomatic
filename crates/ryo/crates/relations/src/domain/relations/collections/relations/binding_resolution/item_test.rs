//! Independent distinct cells guard the public resolution fact layout.
use super::super::interface::RelationsItem as _;
use super::item::{BindingResolution, BindingResolutionRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Integer, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;
type Input = ();
type Value = [Bits; 10];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("distinct identities and depths preserve their columns"),
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
        Bits(17),
        Bits(19),
        Bits(90_194_313_239),
    ]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(BindingResolution::words(BindingResolutionRow {
        reference: NodeId(3),
        event: NodeId(5),
        kind: Symbol(7),
        scope: NodeId(9),
        lexical: NodeId(11),
        depth: Integer(13),
        ordinal: Integer(15),
        context: Symbol(17),
        file: FileId(19),
        span: Span {
            start: Offset(21),
            end: Offset(23),
        },
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
