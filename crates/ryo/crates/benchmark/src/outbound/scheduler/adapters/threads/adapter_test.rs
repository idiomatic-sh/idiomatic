//! The scoped-thread scheduler answers the port's contract.
use super::adapter::SchedulerThreadsAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::scheduler::fixtures::contract::check;

type Input = ();
type Value = bool;
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the scoped-thread scheduler answers the scheduler contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    check(&SchedulerThreadsAdapter);
    Ok(true)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
