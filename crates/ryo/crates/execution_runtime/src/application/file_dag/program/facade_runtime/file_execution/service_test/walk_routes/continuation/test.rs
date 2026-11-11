//! A source class is not a continuation result, and its consumers own immutable answers.
use super::super::fixtures::Residency;
use super::fixtures::{Observed, Walk, exercise};
use super::super::fixtures::semantics::Calls;
use super::super::super::fixtures::Root;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use std::borrow::Cow;
use crate::cases::{Case, CaseName, Expect, Word, run};

const fn n(value: i64) -> Cell {
    Cell::Number(Integer(value))
}

const TRANSFERRED: &[Cell] = &[n(9), n(7), n(7), n(7)];
const SELECTED: &[Cell] = &[n(2), n(2), n(3), n(3)];
const BEFORE_REFUSAL: &[Cell] = &[n(9), n(7)];

/// The consumers' answers over `populations` physical populations of the three source rows:
/// only certified physical populations may share a pass.
const fn consumers(populations: u64) -> Observed {
    Observed {
        continuations_apart: 2,
        equivalent_extends_share_a_region: true,
        other_literal_has_its_own_region: true,
        failed: [
            Root::Failed(OperatingError::WrongOperation),
            Root::Failed(OperatingError::WrongOperation),
        ],
        transferred: Root::complete(TRANSFERRED, 2),
        // Authored repeated kept columns remain repeated.
        selected: Root::complete(SELECTED, 2),
        // Failure stops at its first actual continuation row; equivalent healthy delivery
        // obligations compute once.
        failed_calls: Calls(1),
        appended_calls: Calls(2),
        walk: Walk {
            passes: populations,
            source_rows: 3 * populations,
            predicate_tests: 3 * populations,
            result_slots: populations,
        },
        refused: Some(OperatingError::NotRead),
        delivered_before_refusal: Cow::Borrowed(BEFORE_REFUSAL),
        // Callback retry replays the full committed answer.
        replay_is_the_answer: true,
        collected_retry_is_the_answer: true,
        retries_repeat_no_work: true,
        working_bytes: 0,
        answer_survives_retirement: true,
    }
}

type Input = Residency;
type Value = Observed;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "shared_source_continuation_failure_and_result_consumers_preserve_one_certified_population",
        ),
        word: Word::Succeeds,
        input: || Residency::Stable,
        expect: Expect::Value(consumers(1)),
    },
    Case {
        name: CaseName(
            "uncertified_resident_identity_keeps_equal_source_bytes_in_distinct_populations",
        ),
        word: Word::Succeeds,
        input: || Residency::Unstable,
        expect: Expect::Value(consumers(3)),
    },
];

/// Runs the shared source and its consumers over sources of the residency.
fn subject(residency: Input) -> Result<Value, Failure> {
    exercise(residency)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
