//! What every `scheduler` adapter must pass.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::super::errors::SchedulerError;
use super::super::interface::SchedulerPort;
use super::super::types::{JobCount, JobNumber};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the contract is given: how many jobs to run.
type Input = JobCount;
/// How many jobs ran.
type Value = usize;
/// What a row of the contract expects when it is refused.
type Failure = SchedulerError;

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_job_runs_once"),
        word: Word::Succeeds,
        input: || JobCount(8),
        expect: Expect::Value(8),
    },
    Case {
        name: CaseName("no_jobs_run_no_job"),
        word: Word::Succeeds,
        input: || JobCount(0),
        expect: Expect::Value(0),
    },
];

/// Runs the contract's rows against one implementation: the fake, or an adapter, with `workers` workers.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl SchedulerPort, workers: usize) {
    run(CASES, Word::Succeeds, |count| {
        let ran = std::iter::repeat_with(|| AtomicUsize::new(0))
            .take(usize::try_from(count.0).unwrap_or(usize::MAX))
            .collect::<Vec<_>>();
        let active = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let mut started: Vec<Box<dyn FnMut(JobNumber) + Send + '_>> = Vec::new();
        for _ in 0..workers {
            started.push(Box::new(|index| {
                let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                let _peak_before = peak.fetch_max(now, Ordering::SeqCst);
                if let Some(job) = ran.get(usize::try_from(index.0).unwrap_or(usize::MAX)) {
                    let _runs_before = job.fetch_add(1, Ordering::SeqCst);
                }
                let _active_before = active.fetch_sub(1, Ordering::SeqCst);
            }));
        }
        adapter.run(&mut started, count)?;
        drop(started);
        assert!(
            ran.iter().all(|ran| ran.load(Ordering::SeqCst) == 1),
            "every job runs once"
        );
        assert_eq!(
            active.load(Ordering::SeqCst),
            0,
            "no job runs once the run is over"
        );
        assert!(
            peak.load(Ordering::SeqCst) <= workers,
            "no more jobs run at once than there are workers"
        );
        Ok(ran.iter().map(|ran| ran.load(Ordering::SeqCst)).sum())
    });
}
