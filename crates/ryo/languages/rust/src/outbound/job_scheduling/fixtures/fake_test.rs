//! The `job_scheduling` port's fake runs the port's contract, and runs the jobs in order.

use super::super::contract::{check, turns};
use super::JobSchedulingFake;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the cases asks of the fake: how many jobs to run.
type Input = u32;
/// The turn each job ran at, in the order of the jobs.
type Value = &'static [u32];
type Failure = ();

/// The rows of the fake's own cases: a name, one case word, an input and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_fake_runs_the_jobs_in_order"),
    word: Word::Succeeds,
    input: || 3,
    expect: Expect::Value(&[0, 1, 2]),
}];

/// Calls the fake on one input: the turn each job ran at, in the order of the jobs.
fn subject(count: Input) -> Value {
    let leaked: &'static [u32] = Box::leak(turns(&JobSchedulingFake, count).into_boxed_slice());
    leaked
}

#[test]
fn succeeds() {
    check(&JobSchedulingFake);
    run(CASES, Word::Succeeds, |count| Ok(subject(count)));
}
