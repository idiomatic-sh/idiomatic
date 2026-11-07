//! `static_call` reader cases and the collection contract.
use super::super::fixtures::contract::check;
use super::item::StaticCallItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading;
use ryo::languages::contract::Reading;
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("empty_source_has_no_static_call"),
    word: Word::Succeeds,
    input: || "",
    expect: Expect::Value(Reading {
        rows: &[],
        causes: &[],
    }),
}];
fn subject(source: Input) -> Value {
    check::<StaticCallItem>();
    reading::<StaticCallItem>(source)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| Ok(subject(source)));
}
