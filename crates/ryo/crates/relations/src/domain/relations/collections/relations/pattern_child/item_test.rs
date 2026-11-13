//! Typed item contract for pattern edges.
use super::super::interface::RelationsItem as _;
use super::item::{PatternChild, PatternChildRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Integer, NodeId};
use crate::domain::terms::types::Symbol;
type Input = ();
type Value = [Bits; 5];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("pattern_child_preserves_typed_columns"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(1), Bits(2), Bits(3), Bits(4), Bits(5)]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(PatternChild::words(PatternChildRow {
        parent: NodeId(1),
        child: NodeId(2),
        ordinal: Integer(3),
        field: Symbol(4),
        file: FileId(5),
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
