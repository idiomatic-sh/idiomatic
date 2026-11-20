//! Independent distinct cells guard the public lookup-path fact layout.
use super::super::interface::RelationsItem as _;
use super::item::{LookupScope, LookupScopeRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Integer, NodeId, Offset, Span};
use crate::domain::terms::types::Symbol;
type Input = ();
type Value = [Bits; 7];
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
        Bits(64_424_509_457),
    ]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(LookupScope::words(LookupScopeRow {
        reference: NodeId(3),
        name: Symbol(5),
        scope: NodeId(7),
        depth: Integer(9),
        context: Symbol(11),
        file: FileId(13),
        span: Span {
            start: Offset(15),
            end: Offset(17),
        },
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
