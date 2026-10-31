//! The fake answers the same independent contract as the concrete destination.
use super::EvidenceFake;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::evidence::fixtures::contract::check;
type Input = ();
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("fake honors publication contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    check(&EvidenceFake);
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
