//! The sequential adapter of the scheduler port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::SchedulerSequentialAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::scheduler::errors::SchedulerError;
use crate::outbound::scheduler::fixtures::contract::check;
use crate::outbound::scheduler::interface::SchedulerPort as _;
use crate::outbound::scheduler::types::{JobCount, JobNumber};

/// What a row of the cases asks of the adapter.
#[derive(Clone, Copy)]
enum Scenario {
    /// Runs jobs on the one worker given.
    OneWorker,
    /// Runs jobs when no worker was given.
    NoWorkers,
}

/// What a row of the cases asks of the adapter.
type Input = Scenario;
/// How many workers the adapter names, or a yes or no as a number.
type Value = usize;
type Failure = SchedulerError;

/// The rows of what is specific to this adapter: a name, one case word, an input from the fixtures or a constructor, and the expected
/// value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_one_worker_given_runs_every_job_in_order"),
        word: Word::Succeeds,
        input: || Scenario::OneWorker,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("jobs_cannot_run_when_no_worker_was_given"),
        word: Word::Refuses,
        input: || Scenario::NoWorkers,
        expect: Expect::Error(SchedulerError::NoWorkers),
    },
];

/// Calls the adapter on one input, for its own rows.
fn subject(scenario: Input) -> Result<Value, Failure> {
    let adapter = SchedulerSequentialAdapter;
    match scenario {
        Scenario::OneWorker => {
            let mut seen = Vec::new();
            let mut job = |index: JobNumber| seen.push(index.0);
            let mut workers: [Box<dyn FnMut(JobNumber) + Send + '_>; 1] = [Box::new(&mut job)];
            adapter.run(&mut workers, JobCount(3))?;
            drop(workers);
            Ok(usize::from(seen == [0, 1, 2]) * seen.len())
        }
        Scenario::NoWorkers => adapter.run(&mut [], JobCount(1)).map(|()| 0),
    }
}

#[test]
fn succeeds() {
    check(&SchedulerSequentialAdapter, 1);
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
