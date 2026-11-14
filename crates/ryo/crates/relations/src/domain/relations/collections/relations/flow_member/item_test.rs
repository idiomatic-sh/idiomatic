//! Typed item contract for `flow-member`.
use super::super::interface::RelationsItem as _;
use super::item::{FlowMember, FlowMemberRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::Bits;
use crate::domain::relations::types::{FileId, Integer, NodeId};
type Input = ();
type Value = [Bits; 3];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("flow_member_preserves_typed_columns"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(1), Bits(2), Bits(3)]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(FlowMember::words(FlowMemberRow {
        node: NodeId(1),
        block: Integer(2),
        file: FileId(3),
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
