//! Typed item contract for `flow-edge`.
use super::super::interface::RelationsItem as _;
use super::item::{FlowEdge, FlowEdgeRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::Bits;
use crate::domain::relations::types::{FileId, Integer};
use crate::domain::terms::types::Symbol;
type Input = ();
type Value = [Bits; 5];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("flow_edge_preserves_typed_columns"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(1), Bits(2), Bits(3), Bits(4), Bits(5)]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(FlowEdge::words(FlowEdgeRow {
        source: Integer(1),
        target: Integer(2),
        edge_kind: Symbol(3),
        ordinal: Integer(4),
        file: FileId(5),
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
