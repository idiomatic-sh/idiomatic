//! The rayon scheduler: the contract, then the size of the pool a job runs in, and a pool the system refused.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::scheduler::factory::{pool, rayon};
use crate::outbound::scheduler::fixtures::contract::check;
use crate::outbound::scheduler::{
    errors::SchedulerError,
    interface::SchedulerPort as _,
    types::{WorkerCount, WorkerPool},
};

/// How many workers the pool is given; none for a pool the operating system refused.
type Input = Option<usize>;
/// How many workers the job's pool runs.
type Value = usize;
type Failure = SchedulerError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_job_runs_in_a_pool_of_the_size_given"),
        word: Word::Succeeds,
        input: || Some(3),
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("a_pool_of_no_workers_runs_one"),
        word: Word::Succeeds,
        input: || Some(0),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_pool_the_system_refused_refuses_every_run_with_why"),
        word: Word::Fails,
        input: || None,
        expect: Expect::Refusal(|error| {
            if matches!(error, SchedulerError::Unavailable { reason } if reason == REFUSED) {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// Why the operating system refused the pool, in the row that has it refuse.
const REFUSED: &str = "no threads";

fn subject(workers: Input) -> Result<Value, Failure> {
    let mut size = 0;
    if let Some(workers) = workers {
        rayon(pool(WorkerCount(workers))).run(&mut || size = ::rayon::current_num_threads())?;
    } else {
        // A pool whose build the system refused refuses every run, with why.
        rayon(WorkerPool(Err(SchedulerError::Unavailable {
            reason: REFUSED.to_owned(),
        })))
        .run(&mut || size = ::rayon::current_num_threads())?;
    }
    Ok(size)
}

#[test]
fn succeeds() {
    check(&rayon(pool(WorkerCount(2))));
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
