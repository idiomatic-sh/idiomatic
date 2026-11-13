//! The `facts` port's fake runs the port's contract, and refuses as it is told to.

use super::super::contract::check;
use super::FactsFake;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::interface::FactsPort as _;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::selects;

/// The fake a row calls: one that works, or one told to fail.
#[derive(Clone, Copy)]
enum Fake {
    /// Opens a worker's handle on a fake that works.
    Working,
    /// Opens a worker's handle on a fake told to fail.
    Failing,
}

type Input = Fake;
/// Whether a handle was opened.
type Value = bool;
type Failure = FactsError;

/// The rows of the fake's own cases: a name, one case word, an input and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_fake_that_works_opens_a_handle"),
        word: Word::Succeeds,
        input: || Fake::Working,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("fails_with_its_store_when_told_to"),
        word: Word::Fails,
        input: || Fake::Failing,
        expect: Expect::Refusal(|error| selects!(matches!(error, FactsError::Store { .. }))),
    },
];

/// Calls the fake on one input.
fn subject(fake: Input) -> Result<Value, Failure> {
    match fake {
        Fake::Working => FactsFake::default().worker().map(|_handle| true),
        Fake::Failing => FactsFake::failing().worker().map(|_handle| true),
    }
}

#[test]
fn succeeds() {
    check(&FactsFake::default());
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
