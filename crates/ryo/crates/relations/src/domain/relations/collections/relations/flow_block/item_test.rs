//! Typed item contract for `flow-block`.
use super::super::interface::RelationsItem as _;
use super::item::{FlowBlock, FlowBlockRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::Bits;
use crate::domain::relations::types::{FileId, Flag, Integer};
type Input = ();
type Value = [Bits; 5];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("flow_block_preserves_typed_columns"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(1), Bits(1), Bits(1), Bits(1), Bits(5)]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(FlowBlock::words(FlowBlockRow {
        block: Integer(1),
        reachable: Flag::Raised,
        live: Flag::Raised,
        implicit_return: Flag::Raised,
        file: FileId(5),
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
