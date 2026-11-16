//! What every `scheduler` implementation must pass.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::scheduler::interface::SchedulerPort;
use std::sync::atomic::{AtomicUsize, Ordering};

/// How many tasks a row hands the scheduler.
type Input = usize;
/// How many task runs the scheduler made.
type Value = usize;
type Failure = std::convert::Infallible;

/// The contract's rows.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every task runs exactly once"),
        word: Word::Succeeds,
        input: || 5,
        expect: Expect::Value(5),
    },
    Case {
        name: CaseName("no task is an immediate answer"),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value(0),
    },
];

/// Run the rows against one implementation.
///
/// # Panics
///
/// Panics when an implementation violates a contract row.
pub fn check(port: &impl SchedulerPort) {
    run(CASES, Word::Succeeds, |count| {
        let runs = AtomicUsize::new(0);
        let tasks = (0..count)
            .map(|_| -> Box<dyn FnOnce() + Send + '_> {
                Box::new(|| {
                    let _before = runs.fetch_add(1, Ordering::SeqCst);
                })
            })
            .collect();
        port.run_all(tasks);
        Ok(runs.load(Ordering::SeqCst))
    });
}
