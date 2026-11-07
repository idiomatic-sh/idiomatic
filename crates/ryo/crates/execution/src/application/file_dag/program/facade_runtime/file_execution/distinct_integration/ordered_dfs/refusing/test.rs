//! Ordered graph traversals that refuse name the exact error.
use super::super::super::*;
use super::super::fixtures::{compute, numbers, operation, subject};
use crate::domain::operating::fixtures::ExecutionObservation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::selects;
fn ties_are_rejected_even_without_roots() -> Result<usize, OperatingError> {
    compute(
        &operation(&[], &[]),
        &numbers(1, &[]),
        &numbers(4, &[&[1, 2, 0, 12], &[1, 3, 0, 13]]),
        &mut ExecutionObservation::default(),
    )
    .map(|out| out.count.0)
}
fn duplicate_edges_are_ties() -> Result<usize, OperatingError> {
    compute(
        &operation(&[], &[]),
        &numbers(1, &[&[1]]),
        &numbers(4, &[&[1, 2, 0, 12], &[1, 2, 0, 12]]),
        &mut ExecutionObservation::default(),
    )
    .map(|out| out.count.0)
}
fn malformed_empty_input_is_rejected() -> Result<usize, OperatingError> {
    compute(
        &operation(&[8], &[4]),
        &numbers(1, &[]),
        &numbers(5, &[]),
        &mut ExecutionObservation::default(),
    )
    .map(|out| out.count.0)
}
fn empty_input_kind_mismatch_is_rejected() -> Result<usize, OperatingError> {
    let edges = Table {
        kinds: vec![
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Number,
            CellKind::Number,
        ],
        ..Table::default()
    };
    compute(
        &operation(&[], &[]),
        &numbers(1, &[]),
        &edges,
        &mut ExecutionObservation::default(),
    )
    .map(|out| out.count.0)
}
type Input = fn() -> Result<usize, OperatingError>;
type Value = usize;
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ties_refused_even_with_empty_roots"),
        word: Word::Refuses,
        input: || ties_are_rejected_even_without_roots,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, OperatingError::DuplicateDfsOrder))
        }),
    },
    Case {
        name: CaseName("duplicate_edges_refused_as_order_ties"),
        word: Word::Refuses,
        input: || duplicate_edges_are_ties,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, OperatingError::DuplicateDfsOrder))
        }),
    },
    Case {
        name: CaseName("missing_columns_refused_on_empty_input"),
        word: Word::Refuses,
        input: || malformed_empty_input_is_rejected,
        expect: Expect::Refusal(|error| selects!(matches!(error, OperatingError::Table(..)))),
    },
    Case {
        name: CaseName("entity_kinds_checked_on_empty_input"),
        word: Word::Refuses,
        input: || empty_input_kind_mismatch_is_rejected,
        expect: Expect::Refusal(|error| selects!(matches!(error, OperatingError::ShapeMismatch))),
    },
];
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
