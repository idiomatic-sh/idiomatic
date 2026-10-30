//! The reader obeys the shared collection contract on an empty document.
use super::SyntaxErrorItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::Reading;
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("doctype_only_document_has_no_errors"),
    word: Word::Succeeds,
    input: || "<!doctype html>",
    expect: Expect::Value(Reading {
        rows: &[],
        causes: &[],
    }),
}];
fn subject(source: Input) -> Value {
    super::super::super::fixtures::contract::check::<SyntaxErrorItem>();
    reading::<SyntaxErrorItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
