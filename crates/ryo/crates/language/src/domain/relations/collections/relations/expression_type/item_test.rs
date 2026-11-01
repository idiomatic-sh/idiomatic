//! Expression classifications preserve resolution and source identity.
use super::super::interface::RelationsItem as _;
use super::item::{ExpressionType, ExpressionTypeRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, NodeId, Offset, Span};
type Input = ();
type Value = [Bits; 5];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("expression type row retains resolution and type"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(17), Bits(23), Bits(7), Bits(47_244_640_269)]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(ExpressionType::words(ExpressionTypeRow {
        id: NodeId(3),
        resolution: crate::domain::terms::types::Symbol(17),
        type_: crate::domain::terms::types::Symbol(23),
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
