//! Independent distinct cells guard the public annotation-timing fact layout.
use super::super::interface::RelationsItem as _;
use super::item::{AnnotationEvaluation, AnnotationEvaluationRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, NodeId};
use crate::domain::terms::types::Symbol;
type Input = ();
type Value = [Bits; 5];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("distinct identities and timings preserve their columns"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(5), Bits(7), Bits(9), Bits(11)]),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    Ok(AnnotationEvaluation::words(AnnotationEvaluationRow {
        id: NodeId(3),
        root: NodeId(5),
        evaluation: Symbol(7),
        mode: Symbol(9),
        file: FileId(11),
    }))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
