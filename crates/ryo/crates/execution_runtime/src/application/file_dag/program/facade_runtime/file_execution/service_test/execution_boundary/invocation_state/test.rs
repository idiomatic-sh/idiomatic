//! Chained registered evaluations reuse downstream invocation state and release their peaks.
use super::fixtures::{Observed, Program, exercise};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use std::borrow::Cow;

type Input = Program;
type Value = Observed;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("chained_registered_evaluations_reuse_downstream_invocation_state"),
        word: Word::Succeeds,
        input: || Program::Chained,
        expect: Expect::Value(Observed::Chained {
            starts: Cow::Borrowed(&[0, 1, 2]),
            rows: Cow::Borrowed(&[
                Cell::Number(Integer(1)),
                Cell::Number(Integer(2)),
                Cell::Number(Integer(3)),
            ]),
        }),
    },
    Case {
        name: CaseName("partition_observations_keep_large_then_small_peak_and_release"),
        word: Word::Succeeds,
        input: || Program::Partitioned,
        expect: Expect::Value(Observed::Closure {
            rows: 529,
            partition_rows: 32,
            capacity_holds_both_peaks: true,
        }),
    },
];

/// Runs the program on one file.
fn subject(program: Input) -> Result<Value, Failure> {
    Ok(exercise(program))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
