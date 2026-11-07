//! The system adapter of the clock port runs the contract.

use super::adapter::ClockSystemAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::clock::fixtures::contract::check;

type Input = ();
/// Whether the contract held.
type Value = bool;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("today_is_a_day_of_the_machines_clock"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

fn subject((): Input) -> Value {
    check(&ClockSystemAdapter);
    true
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
