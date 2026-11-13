//! The rayon adapter of the job scheduling port: the port's contract rows run through it, then the cases specific to it.

use std::sync::atomic::{AtomicBool, Ordering};

use super::adapter::JobSchedulingRayonAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::job_scheduling::fixtures::contract::check;
use crate::outbound::job_scheduling::interface::JobSchedulingPort as _;

/// What a row of the cases asks of the adapter: how many threads the pool it is called inside has.
type Input = usize;
/// Whether every job ran on a worker of that pool.
type Value = bool;
type Failure = ();

/// The rows of what is specific to this adapter: a name, one case word, an input from the fixtures or a constructor, and the expected
/// value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_jobs_run_on_the_pool_the_caller_runs_on"),
    word: Word::Succeeds,
    input: || 2,
    expect: Expect::Value(true),
}];

/// Runs jobs with the adapter inside a pool of `threads`, and answers whether each ran on one of that pool's workers; a pool that
/// cannot be built answers that they did not.
fn subject(threads: Input) -> Value {
    let Ok(pool) = rayon::ThreadPoolBuilder::new().num_threads(threads).build() else {
        return false;
    };
    let inside = AtomicBool::new(true);
    pool.install(|| {
        let inside = &inside;
        let jobs = (0..16)
            .map(|_| {
                let job: Box<dyn FnOnce() + Send + '_> = Box::new(move || {
                    if rayon::current_thread_index().is_none_or(|index| index >= threads) {
                        inside.store(false, Ordering::Relaxed);
                    }
                });
                job
            })
            .collect();
        JobSchedulingRayonAdapter.run(crate::outbound::job_scheduling::types::Jobs(jobs));
    });
    inside.load(Ordering::Relaxed)
}

#[test]
fn succeeds() {
    check(&JobSchedulingRayonAdapter);
    run(CASES, Word::Succeeds, |threads| Ok(subject(threads)));
}
