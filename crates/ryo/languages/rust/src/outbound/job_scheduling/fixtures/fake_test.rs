//! The `job_scheduling` port's fake runs the port's contract, and runs the jobs in order.

use std::sync::{Mutex, PoisonError};

use super::super::contract::check;
use super::JobSchedulingFake;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::job_scheduling::interface::JobSchedulingPort as _;
use crate::outbound::job_scheduling::types::{JobIndex, ScheduledJobCount};

/// What a row of the cases asks of the fake: how many jobs to run.
type Input = ScheduledJobCount;
/// The jobs, in the order they ran.
type Value = &'static [u32];
type Failure = ();

/// The rows of the fake's own cases: a name, one case word, an input and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_fake_runs_the_jobs_in_order"),
    word: Word::Succeeds,
    input: || ScheduledJobCount(3),
    expect: Expect::Value(&[0, 1, 2]),
}];

/// Calls the fake on one input: the jobs in the order they ran.
fn subject(count: Input) -> Value {
    let ran: Mutex<Vec<u32>> = Mutex::new(Vec::new());
    JobSchedulingFake.run(count, &|job: JobIndex| {
        ran.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(job.0);
    });
    let ran = ran.into_inner().unwrap_or_else(PoisonError::into_inner);
    let leaked: &'static [u32] = Box::leak(ran.into_boxed_slice());
    leaked
}

#[test]
fn succeeds() {
    check(&JobSchedulingFake);
    run(CASES, Word::Succeeds, |count| Ok(subject(count)));
}
