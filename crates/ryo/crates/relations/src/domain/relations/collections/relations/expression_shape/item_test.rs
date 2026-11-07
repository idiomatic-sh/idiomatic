//! Distinct occurrence and representative IDs protect the shared row contract.
use super::super::interface::RelationsItem as _;
use super::item::{ExpressionShape, ExpressionShapeRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, NodeId, Offset, Span};
type Input = ();
type Value = [Bits; 4];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("opaque class retains original occurrence span"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(11), Bits(7), Bits(47_244_640_269)]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(ExpressionShape::words(ExpressionShapeRow {
        id: NodeId(3),
        shape: NodeId(11),
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
