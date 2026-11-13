//! What every `job_scheduling` adapter must pass.

use std::sync::atomic::{AtomicU32, Ordering};

use super::super::interface::JobSchedulingPort;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the contract is given: how many jobs to run.
type Input = u32;
/// The job each run was for, in order: every job once.
type Value = &'static [u32];
/// What a row of the contract expects when it is refused: running jobs is never refused.
type Failure = ();

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_job_runs_once"),
        word: Word::Succeeds,
        input: || 8,
        expect: Expect::Value(&[0, 1, 2, 3, 4, 5, 6, 7]),
    },
    Case {
        name: CaseName("no_jobs_run_no_job"),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value(&[]),
    },
];

/// Runs `count` jobs through one implementation, each writing only its own turn: for each job that ran, in the order of the jobs, the
/// turn it ran at, counting from zero.
pub(crate) fn turns(adapter: &impl JobSchedulingPort, count: u32) -> Vec<u32> {
    let next = AtomicU32::new(0);
    let mut turns: Vec<Option<u32>> = (0..count).map(|_| None).collect();
    let jobs = turns
        .iter_mut()
        .map(|turn| {
            let next = &next;
            {
                let job: Box<dyn FnOnce() + Send + '_> =
                    Box::new(move || *turn = Some(next.fetch_add(1, Ordering::Relaxed)));
                job
            }
        })
        .collect();
    adapter.run(super::super::types::Jobs(jobs));
    turns.into_iter().flatten().collect()
}

/// Runs the contract's rows against one implementation: the fake, or an adapter.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub(crate) fn check(adapter: &impl JobSchedulingPort) {
    run(CASES, Word::Succeeds, |count| {
        let mut ran = turns(adapter, count);
        ran.sort_unstable();
        let leaked: &'static [u32] = Box::leak(ran.into_boxed_slice());
        Ok(leaked)
    });
}
