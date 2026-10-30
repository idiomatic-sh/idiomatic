//! The reader obeys the shared collection contract on an empty document.
use super::SyntaxChildItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = ();
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("reader_satisfies_shared_collection_contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
fn subject((): Input) -> Value {
    super::super::super::fixtures::contract::check::<SyntaxChildItem>();
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |source| {
        subject(source);
        Ok(())
    });
}
