//! The `memory` port's fake runs the port's contract, and fails as it is told to.

use super::super::contract::check;
use super::MemoryFake;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::memory::errors::MemoryError;
use crate::outbound::memory::interface::MemoryPort as _;

/// What a row of the cases asks of the fake.
#[derive(Clone, Copy)]
enum Scenario {
    /// The memory a fake with a gigabyte to spare reports.
    Spare,
    /// The memory a fake told to fail reports.
    Failing,
    /// The memory a fake given none reports.
    NoneToSpare,
}

/// What a row of the cases asks of the fake.
type Input = Scenario;
/// A yes or no as a number.
type Value = usize;
type Failure = MemoryError;

/// The rows of the fake's own cases: a name, one case word, an input and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_fake_with_memory_to_spare_reports_it"),
        word: Word::Succeeds,
        input: || Scenario::Spare,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_fake_given_no_memory_to_spare_says_none_is_available"),
        word: Word::Refuses,
        input: || Scenario::NoneToSpare,
        expect: Expect::Error(MemoryError::NoneAvailable),
    },
    Case {
        name: CaseName("fails_when_the_machine_cannot_say"),
        word: Word::Fails,
        input: || Scenario::Failing,
        expect: Expect::Refusal(|error| selects(matches!(error, MemoryError::Unreadable { .. }))),
    },
];

/// Calls the fake on one input.
fn subject(scenario: Input) -> Result<Value, Failure> {
    let fake = match scenario {
        Scenario::Spare => MemoryFake::default(),
        Scenario::Failing => MemoryFake {
            fails: true,
            ..MemoryFake::default()
        },
        Scenario::NoneToSpare => MemoryFake {
            available: 0,
            ..MemoryFake::default()
        },
    };
    fake.budget().map(|bytes| usize::from(bytes.0 > 0))
}

#[test]
fn succeeds() {
    check(&MemoryFake::default());
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
