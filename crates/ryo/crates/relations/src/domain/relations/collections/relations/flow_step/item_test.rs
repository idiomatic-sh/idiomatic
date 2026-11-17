//! Typed item contract for `flow-step`.
use super::super::interface::RelationsItem as _;
use super::item::{FlowStep, FlowStepRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::Bits;
use crate::domain::relations::types::{FileId, Integer, NodeId};
use crate::domain::terms::types::Symbol;
type Input = ();
type Value = [Bits; 5];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("flow_step_preserves_typed_columns"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(1), Bits(2), Bits(3), Bits(4), Bits(5)]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(FlowStep::words(FlowStepRow {
        block: Integer(1),
        ordinal: Integer(2),
        step_kind: Symbol(3),
        node: NodeId(4),
        file: FileId(5),
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
