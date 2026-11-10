//! Function outcome rows retain the definition node and its source location.
use super::super::interface::RelationsItem as _;
use super::item::{FunctionTerminal, FunctionTerminalRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, NodeId, Offset, Span};
type Input = ();
type Value = [Bits; 4];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("terminal outcome row retains identity"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(17), Bits(7), Bits(47_244_640_269)]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(FunctionTerminal::words(FunctionTerminalRow {
        id: NodeId(3),
        outcome: crate::domain::terms::types::Symbol(17),
        file: FileId(7),
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
