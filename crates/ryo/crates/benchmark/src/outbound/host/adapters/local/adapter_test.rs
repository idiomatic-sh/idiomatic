//! The local probe answers the port's contract.
use super::adapter::HostLocalAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::host::fixtures::contract::check;

type Input = ();
type Value = bool;
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the local probe answers the host contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    check(&HostLocalAdapter);
    Ok(true)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
