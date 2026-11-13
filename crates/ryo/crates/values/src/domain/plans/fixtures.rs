//! The test support of the `plans` concept: the plan reads the tests of other crates make.

use super::service;
use super::types::Operation;
use crate::domain::plan_parts::types::{Input, Inputs};
use crate::domain::query_operators::types::OperationKind;

/// The inputs an operation reads.
#[must_use]
pub fn inputs_of(operation: &Operation) -> Inputs {
    service::inputs_of(operation)
}

/// The kind of an operation.
#[must_use]
pub fn kind_of(operation: &Operation) -> OperationKind {
    service::kind_of(operation)
}

/// The inputs of an operation that its inputs name.
#[must_use]
pub fn named(inputs: &Inputs) -> &[Input] {
    service::named(inputs)
}
