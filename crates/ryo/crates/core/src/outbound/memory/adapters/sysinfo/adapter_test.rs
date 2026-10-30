//! The sysinfo adapter of the memory port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::MemorySysinfoAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::memory::errors::MemoryError;
use crate::outbound::memory::fixtures::contract::check;
use crate::outbound::memory::interface::MemoryPort as _;
use ryo_values::domain::runs::types::{Budget, MemoryBytes};

/// What a row of the cases asks of the adapter.
#[derive(Clone, Copy)]
enum Scenario {
    /// Whether the machine says how much memory a run may use and how much the process holds, two times running.
    ReadTwice,
    /// The budget of a caller that asked for one byte.
    AskedForLess,
    /// The budget of a caller that asked for more than any machine has.
    AskedForMore,
}

/// What a row of the cases asks of the adapter.
type Input = Scenario;
/// A yes or no as a number, or a number of bytes.
type Value = u64;
type Failure = MemoryError;

/// The rows of what is specific to this adapter: a name, one case word, an input from the fixtures or a constructor, and the expected
/// value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_machine_says_what_a_run_may_use_and_what_the_process_holds_each_time"),
        word: Word::Succeeds,
        input: || Scenario::ReadTwice,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_caller_that_asked_for_less_than_the_machine_has_gets_what_it_asked"),
        word: Word::Succeeds,
        input: || Scenario::AskedForLess,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_caller_that_asked_for_more_than_the_machine_has_is_refused"),
        word: Word::Refuses,
        input: || Scenario::AskedForMore,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, MemoryError::MoreThanAvailable { .. }))
        }),
    },
];

/// Calls the adapter on one input, for its own rows.
fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::ReadTwice => {
            let adapter = MemorySysinfoAdapter::new(Budget::Machine);
            let first = adapter.budget()?.0 > 0 && adapter.resident_bytes()?.0 > 0;
            let second = adapter.budget()?.0 > 0 && adapter.resident_bytes()?.0 > 0;
            Ok(u64::from(first && second))
        }
        Scenario::AskedForLess => Ok(MemorySysinfoAdapter::new(Budget::Bytes(MemoryBytes(1)))
            .budget()?
            .0),
        Scenario::AskedForMore => Ok(MemorySysinfoAdapter::new(Budget::Bytes(MemoryBytes(
            u64::MAX,
        )))
        .budget()?
        .0),
    }
}

#[test]
fn succeeds() {
    check(&MemorySysinfoAdapter::new(Budget::Machine));
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
