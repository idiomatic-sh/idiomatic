//! A row of the shared relation keeps its file, element, annotation and the two words beside them.
use super::super::interface::RelationsItem as _;
use super::item::{AnnotationEvaluation, AnnotationEvaluationRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, NodeId};
use crate::domain::terms::types::Symbol;

type Input = ();
type Value = [Bits; 5];
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_row_holds_its_element_annotation_and_words_beside_its_file"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(1), Bits(19), Bits(23), Bits(7)]),
}];

fn subject(_: Input) -> Result<Value, Failure> {
    let words = AnnotationEvaluation::words(AnnotationEvaluationRow {
        id: NodeId(3),
        root: NodeId(1),
        evaluation: Symbol(19),
        mode: Symbol(23),
        file: FileId(7),
    });
    Ok(words)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
