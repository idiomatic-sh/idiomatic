//! The `runs` service's cases, on values alone.

use super::service::workers_to_start;
use super::types::{MachineWorkers, StartedWorkers, WorkersAsked};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the cases asks of the service: the workers asked for and the machine's.
#[derive(Clone, Copy)]
struct Asking {
    asked: usize,
    machine: u32,
}

type Input = Asking;
/// The workers started.
type Value = StartedWorkers;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("none_asked_starts_the_machines"),
        word: Word::Succeeds,
        input: || Asking {
            asked: 0,
            machine: 8,
        },
        expect: Expect::Value(StartedWorkers(8)),
    },
    Case {
        name: CaseName("fewer_than_the_machine_starts_what_was_asked"),
        word: Word::Succeeds,
        input: || Asking {
            asked: 3,
            machine: 8,
        },
        expect: Expect::Value(StartedWorkers(3)),
    },
    Case {
        name: CaseName("more_than_the_machine_starts_the_machines"),
        word: Word::Succeeds,
        input: || Asking {
            asked: 64,
            machine: 8,
        },
        expect: Expect::Value(StartedWorkers(8)),
    },
];

fn subject(asking: Input) -> Result<Value, Failure> {
    Ok(workers_to_start(
        WorkersAsked(asking.asked),
        MachineWorkers(asking.machine),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
