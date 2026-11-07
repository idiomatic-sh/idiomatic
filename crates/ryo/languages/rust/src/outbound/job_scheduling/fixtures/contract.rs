//! What every `job_scheduling` adapter must pass.

use std::sync::{Mutex, PoisonError};

use super::super::interface::JobSchedulingPort;
use super::super::types::{JobIndex, ScheduledJobCount};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the contract is given: how many jobs to run.
type Input = ScheduledJobCount;
/// The job each run was for, in order: every job once.
type Value = &'static [u32];
/// What a row of the contract expects when it is refused: running jobs is never refused.
type Failure = ();

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_job_runs_once"),
        word: Word::Succeeds,
        input: || ScheduledJobCount(8),
        expect: Expect::Value(&[0, 1, 2, 3, 4, 5, 6, 7]),
    },
    Case {
        name: CaseName("no_jobs_run_no_job"),
        word: Word::Succeeds,
        input: || ScheduledJobCount(0),
        expect: Expect::Value(&[]),
    },
];

/// Runs the contract's rows against one implementation: the fake, or an adapter.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub(crate) fn check(adapter: &impl JobSchedulingPort) {
    run(CASES, Word::Succeeds, |count| {
        let ran: Mutex<Vec<u32>> = Mutex::new(Vec::new());
        adapter.run(count, &|job: JobIndex| {
            ran.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(job.0);
        });
        let mut jobs = ran.into_inner().unwrap_or_else(PoisonError::into_inner);
        jobs.sort_unstable();
        let leaked: &'static [u32] = Box::leak(jobs.into_boxed_slice());
        Ok(leaked)
    });
}
