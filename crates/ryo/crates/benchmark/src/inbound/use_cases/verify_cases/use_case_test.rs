//! Actual public suite verification over independent source inputs and real child processes.
use super::fixtures::{Refusal, Scenario, exercise};
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = Scenario;
type Value = usize;
type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("two authored cases execute through the actual public operation"),
        word: Word::Succeeds,
        input: || Scenario::Complete,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("missing sources refuse before any process executes"),
        word: Word::Refuses,
        input: || Scenario::Missing,
        expect: Expect::Error(Refusal::MissingSource),
    },
    Case {
        name: CaseName("removed query refuses before either contestant executes"),
        word: Word::Refuses,
        input: || Scenario::Unaccounted,
        expect: Expect::Error(Refusal::Unaccounted),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    exercise(input)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
