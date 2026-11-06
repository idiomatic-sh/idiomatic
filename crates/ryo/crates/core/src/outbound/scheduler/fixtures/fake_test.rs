//! The `scheduler` port's fake runs the port's contract, and refuses as it is told to.

use super::super::contract::check;
use super::{CountingSchedulerFake, SchedulerFake};
use crate::cases::{Case, CaseName, Expect, Word, run, Matched, selects};
use crate::outbound::scheduler::errors::SchedulerError;
use crate::outbound::scheduler::interface::SchedulerPort as _;
use crate::outbound::scheduler::types::{JobCount, JobNumber};

/// What a row of the cases asks of the fake.
#[derive(Clone, Copy)]
enum Scenario {
    /// Runs jobs on the one worker of a fake that works.
    Works,
    /// Runs jobs on a fake told to fail.
    Failing,
    /// Runs no job on the fake that counts the blocks each job allocates.
    Counting,
    /// Runs jobs when no worker was given.
    NoWorkers,
}

/// What a row of the cases asks of the fake.
type Input = Scenario;
/// How many workers the fake names.
type Value = usize;
type Failure = SchedulerError;

/// The rows of the fake's own cases: a name, one case word, an input and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_fake_that_works_runs_jobs_on_its_one_worker"),
        word: Word::Succeeds,
        input: || Scenario::Works,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_counting_fake_that_runs_no_job_has_counted_none"),
        word: Word::Succeeds,
        input: || Scenario::Counting,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("jobs_cannot_run_when_no_worker_was_given"),
        word: Word::Refuses,
        input: || Scenario::NoWorkers,
        expect: Expect::Error(SchedulerError::NoWorkers),
    },
    Case {
        name: CaseName("fails_when_the_workers_cannot_be_started"),
        word: Word::Fails,
        input: || Scenario::Failing,
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(error, SchedulerError::Unavailable { .. })))
        }),
    },
];

/// Calls the fake on one input.
fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Works => {
            let fake = SchedulerFake::default();
            let mut workers: Vec<Box<dyn FnMut(JobNumber) + Send>> = vec![Box::new(|_| {})];
            fake.run(&mut workers, JobCount(1)).map(|()| 1)
        }
        Scenario::Failing => {
            let mut workers: Vec<Box<dyn FnMut(JobNumber) + Send>> = vec![Box::new(|_| {})];
            SchedulerFake { fails: true }
                .run(&mut workers, JobCount(1))
                .map(|()| 0)
        }
        Scenario::Counting => {
            let fake = CountingSchedulerFake::default();
            let mut workers: Vec<Box<dyn FnMut(JobNumber) + Send>> = vec![Box::new(|_| {})];
            fake.run(&mut workers, JobCount(0))
                .map(|()| fake.blocks.lock().map_or(usize::MAX, |blocks| blocks.len()))
        }
        Scenario::NoWorkers => SchedulerFake::default()
            .run(&mut [], JobCount(1))
            .map(|()| 0),
    }
}

#[test]
fn succeeds() {
    check(&SchedulerFake::default(), 1);
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
