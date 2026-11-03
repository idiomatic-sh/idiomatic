//! Independent distinct cells guard the public binding fact layout.
use super::super::interface::RelationsItem as _;
use super::item::{BindingShadow, BindingShadowRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, NodeId};
use crate::domain::terms::types::Symbol;
type Input = ();
type Value = [Bits; 7];
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
    ]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(BindingShadow::words(BindingShadowRow {
        binding: NodeId(3),
        previous: NodeId(5),
        scope: NodeId(7),
        previous_scope: NodeId(9),
        phase: NodeId(11),
        relationship: Symbol(13),
        file: FileId(15),
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
