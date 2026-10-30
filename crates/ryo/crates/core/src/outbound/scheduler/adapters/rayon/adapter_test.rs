//! The rayon adapter of the scheduler port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::SchedulerRayonAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::scheduler::errors::SchedulerError;
use crate::outbound::scheduler::fixtures::contract::check;
use crate::outbound::scheduler::interface::SchedulerPort as _;
use crate::outbound::scheduler::types::JobCount;

/// What a row of the cases asks of the adapter.
#[derive(Clone, Copy)]
enum Scenario {
    /// How many workers a run starts when one is asked.
    Started,
    /// Runs jobs when no worker was given.
    NoWorkers,
    /// Runs jobs over a pool that could not be built.
    NoPool,
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
        name: CaseName("one_worker_asked_is_one_worker_started"),
        word: Word::Succeeds,
        input: || Scenario::Started,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("jobs_cannot_run_over_a_pool_that_could_not_be_built"),
        word: Word::Refuses,
        input: || Scenario::NoPool,
        expect: Expect::Error(SchedulerError::Unavailable {
            reason: String::new(),
        }),
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
    let adapter = SchedulerRayonAdapter::new(pool(3));
    match scenario {
        Scenario::Started => Ok(usize::try_from(
            crate::outbound::scheduler::factory::workers(
                ryo_values::domain::runs::types::WorkersAsked(1),
            )
            .0,
        )
        .unwrap_or(0)),
        Scenario::NoWorkers => adapter.run(&mut [], JobCount(1)).map(|()| 0),
        Scenario::NoPool => {
            let unbuilt = Err(SchedulerError::Unavailable {
                reason: String::new(),
            });
            let mut job = |_| {};
            let mut workers: [Box<dyn FnMut(_) + Send + '_>; 1] = [Box::new(&mut job)];
            SchedulerRayonAdapter::new(unbuilt)
                .run(&mut workers, JobCount(1))
                .map(|()| 0)
        }
    }
}

/// A pool of `threads` threads, as an edge builds it.
fn pool(threads: usize) -> Result<std::sync::Arc<rayon::ThreadPool>, SchedulerError> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .map(std::sync::Arc::new)
        .map_err(|error| SchedulerError::Unavailable {
            reason: error.to_string(),
        })
}

#[test]
fn succeeds() {
    check(&SchedulerRayonAdapter::new(pool(2)), 2);
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
