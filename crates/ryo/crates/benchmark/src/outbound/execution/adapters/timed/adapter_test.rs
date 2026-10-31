//! Resource instrumentation preserves the execution port's process observations.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::execution::fixtures::contract::check;
type Input = ();
type Value = ();
type Failure = crate::outbound::execution::fixtures::contract::LaunchFailure;
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("timed execution honors process contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    check(&crate::outbound::execution::factory::timed());
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
