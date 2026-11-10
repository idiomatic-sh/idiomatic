//! The in-memory protocol double satisfies the same observable contract.
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = ();
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("fake preserves collector protocol"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
#[expect(
    clippy::unnecessary_wraps,
    reason = "the shared case runner requires a Result-returning subject for every outcome word"
)]
fn subject(_: Input) -> Result<Value, Failure> {
    super::super::contract::check(&super::CollectionFake::default());
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
