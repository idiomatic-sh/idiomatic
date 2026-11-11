//! The support of the `refusing` cases.
use super::super::fixtures::{compute, numbers, operation};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::fixtures::ExecutionObservation;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::rows::types::Table;

/// The same two edges as the ties below, with distinct orders, from root 1: no tie to refuse.
pub(super) fn distinct_orders_are_not_ties() -> Result<usize, OperatingError> {
    compute(
        &operation(&[], &[]),
        &numbers(1, &[&[1]]),
        &numbers(4, &[&[1, 2, 0, 12], &[1, 3, 1, 13]]),
        &mut ExecutionObservation::default(),
    )
    .map(|out| out.count.0)
}

pub(super) fn ties_are_rejected_even_without_roots() -> Result<usize, OperatingError> {
    compute(
        &operation(&[], &[]),
        &numbers(1, &[]),
        &numbers(4, &[&[1, 2, 0, 12], &[1, 3, 0, 13]]),
        &mut ExecutionObservation::default(),
    )
    .map(|out| out.count.0)
}

pub(super) fn duplicate_edges_are_ties() -> Result<usize, OperatingError> {
    compute(
        &operation(&[], &[]),
        &numbers(1, &[&[1]]),
        &numbers(4, &[&[1, 2, 0, 12], &[1, 2, 0, 12]]),
        &mut ExecutionObservation::default(),
    )
    .map(|out| out.count.0)
}

pub(super) fn malformed_empty_input_is_rejected() -> Result<usize, OperatingError> {
    compute(
        &operation(&[8], &[4]),
        &numbers(1, &[]),
        &numbers(5, &[]),
        &mut ExecutionObservation::default(),
    )
    .map(|out| out.count.0)
}

pub(super) fn empty_input_kind_mismatch_is_rejected() -> Result<usize, OperatingError> {
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
