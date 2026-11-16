//! The Linux meter answers the port's contract.
use super::adapter::ProcessMeterLinuxAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::process_meter::fixtures::contract::check;

type Input = ();
type Value = bool;
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the Linux meter answers the process meter contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    check(&ProcessMeterLinuxAdapter);
    Ok(true)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
