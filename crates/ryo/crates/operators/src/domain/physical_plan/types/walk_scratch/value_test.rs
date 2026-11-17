//! A worker's walk scratch starts empty: no walk has met, reached or kept anything before its first file.
use super::value::WalkScratch;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PredicateOutcome;

/// The walk scratch a worker starts with.
type Input = Start;
type Value = Observation;
type Failure = ();

/// How a worker's walk scratch is made.
#[derive(Clone, Copy, Debug)]
enum Start {
    /// As a new worker makes it.
    New,
}

/// What a walk scratch holds.
#[derive(Debug, PartialEq, Eq)]
enum Observation {
    /// Nothing met, left to expand, reached, run, started, keyed, answered or written.
    Empty,
    /// Something.
    Holding,
}

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_new_walk_scratch_holds_nothing"),
    word: Word::Succeeds,
    input: || Start::New,
    expect: Expect::Value(Observation::Empty),
}];

fn subject(start: Input) -> Result<Value, Failure> {
    let walk = match start {
        Start::New => WalkScratch::default(),
    };
    let empty = walk.visited.is_empty() == PredicateOutcome::Holds
        && walk.frontier.is_empty()
        && walk.reached.is_empty()
        && walk.runs.is_empty()
        && walk.starts.is_empty()
        && walk.columns.seed.is_empty()
        && walk.columns.near.is_empty()
        && walk.columns.far.is_empty()
        && walk.rows.count.0 == 0
        && walk.entities.is_empty()
        && walk.next.is_empty()
        && walk.written.is_empty();
    Ok(if empty {
        Observation::Empty
    } else {
        Observation::Holding
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
