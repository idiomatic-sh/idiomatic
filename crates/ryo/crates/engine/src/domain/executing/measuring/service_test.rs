//! The measuring hooks' cases: in a measuring worker a unit's uses, evaluations and rows are counted and the units
//! running are kept, innermost last; in a worker that does not measure every hook does nothing.

use super::service::{entered, evaluated, left, used};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::WordSlot;
use crate::domain::scratch::types::{LoopState, Measuring};
use ryo_relations::domain::relations::types::Count;

/// Whether the worker measures.
#[derive(Clone, Copy, Debug)]
enum Worker {
    /// It measures.
    Measuring,
    /// It does not.
    Plain,
}

type Input = Worker;
/// Of unit 2 after two uses and two evaluations (of one key and 3 rows, then of 4 keys and no row): its uses, keys and
/// rows, and its rows in the file; then how many units ran at the deepest (unit 2 inside unit 1) and how many run at the
/// end; 9s for a worker that holds no counts.
type Value = [usize; 6];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_measuring_worker_counts_a_units_uses_evaluations_and_rows"),
        word: Word::Succeeds,
        input: || Worker::Measuring,
        expect: Expect::Value([2, 5, 3, 3, 2, 0]),
    },
    Case {
        name: CaseName("a_worker_that_does_not_measure_counts_nothing"),
        word: Word::Succeeds,
        input: || Worker::Plain,
        expect: Expect::Value([9; 6]),
    },
];

fn subject(worker: Input) -> Result<Value, Failure> {
    let mut state = LoopState {
        measure: match worker {
            Worker::Measuring => Some(Box::new(Measuring::default())),
            Worker::Plain => None,
        },
        ..LoopState::default()
    };
    let unit = WordSlot(2);
    entered(&mut state, WordSlot(1));
    used(&mut state, unit);
    entered(&mut state, unit);
    let deepest = state.measure.as_deref().map(|held| held.running.len());
    evaluated(&mut state, unit, Count(1), Count(3));
    left(&mut state);
    used(&mut state, unit);
    evaluated(&mut state, unit, Count(4), Count(0));
    left(&mut state);
    let Some(measure) = state.measure.as_deref() else {
        return Ok([9; 6]);
    };
    let at = |column: &Vec<Count>| column.get(2).map_or(0, |held| held.0);
    Ok([
        at(&measure.calls),
        at(&measure.keys),
        at(&measure.rows),
        measure.file.get(2).map_or(0, |held| held.rows.0),
        deepest.unwrap_or(0),
        measure.running.len(),
    ])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
