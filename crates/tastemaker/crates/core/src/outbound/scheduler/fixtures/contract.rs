//! What every scheduler must pass: a job runs once, and the parallel work it starts runs in the scheduler's own pool,
//! never the caller's.
use super::super::{errors::SchedulerError, interface::SchedulerPort};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row asks: how many items the job's parallel work covers.
type Input = usize;
/// Whether the job ran once, and its work covered every item in a pool other than the caller's.
type Value = bool;
/// The port's own failure.
type Failure = SchedulerError;

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_job_runs_once_in_the_schedulers_own_pool"),
        word: Word::Succeeds,
        input: || 8,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_job_with_no_work_runs_once"),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value(true),
    },
];

/// Runs the contract's rows against one implementation.
/// # Panics
/// Panics when an implementation violates the shared contract.
pub fn check(adapter: &impl SchedulerPort) {
    use rayon::prelude::*;
    run(CASES, Word::Succeeds, |items| {
        // A caller's pool of a size no scheduler under test has.
        let foreign = rayon::ThreadPoolBuilder::new()
            .num_threads(97)
            .build()
            .map_err(|error| SchedulerError::Unavailable {
                reason: error.to_string(),
            })?;
        let mut runs = 0_usize;
        let mut covered = Vec::new();
        let mut own_pool = false;
        foreign.install(|| {
            adapter.run(&mut || {
                runs += 1;
                own_pool = rayon::current_num_threads() != 97;
                covered = (0..items).into_par_iter().collect::<Vec<_>>();
            })
        })?;
        Ok(runs == 1 && own_pool && covered == (0..items).collect::<Vec<_>>())
    });
}
