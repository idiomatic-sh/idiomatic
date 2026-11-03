//! Distinct values prove the shared row layout, including ordered version components.
use super::super::interface::RelationsItem as _;
use super::item::{QualifiedName, QualifiedNameRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    relations::types::{Bits, FileId, Integer, NodeId, Offset, Span},
    terms::types::Symbol,
};
type Input = ();
type Value = [Bits; 7];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("canonical name row preserves provenance and numeric version components"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([
        Bits(3),
        Bits(5),
        Bits(7),
        Bits(3),
        Bits(11),
        Bits(17),
        Bits(47_244_640_269),
    ]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(QualifiedName::words(QualifiedNameRow {
        id: NodeId(3),
        path: Symbol(5),
        provenance: Symbol(7),
        since_major: Integer(3),
        since_minor: Integer(11),
        file: FileId(17),
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
