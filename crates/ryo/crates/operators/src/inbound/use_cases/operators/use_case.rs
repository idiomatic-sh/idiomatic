//! Inbound implementation delegates projection to the pure domain algorithm.
use super::interface::OperatorsUseCase;
use crate::domain::contract::types::{OperatorAnswer, OperatorCall, PortableOperatorFault};
use crate::domain::project::service;

/// Stateless capability implementation; the adapter is composed once by the engine.
#[derive(Clone, Copy, Debug)]
pub struct OperatorsUseCaseImpl;

impl OperatorsUseCaseImpl {
    /// A stateless semantic capability composed once at an edge.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl OperatorsUseCase for OperatorsUseCaseImpl {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        match call {
            OperatorCall::EqualityJoin {
                left,
                left_on,
                output,
                view,
                driver,
                key,
                target,
            } => {
                crate::domain::equality_join::service::join(
                    left, left_on, output, view, driver, key, target,
                )?;
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::Project {
                row,
                columns,
                target,
            } => {
                service::project(row, columns, target)?;
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::Append {
                row,
                appended,
                target,
            } => {
                crate::domain::append::service::append(row, appended, target);
                Ok(OperatorAnswer::Written)
            }
            OperatorCall::SpanBounds { row, column } => {
                crate::domain::span_bounds::service::bounds(row, column)
                    .map(OperatorAnswer::SpanBounds)
            }
            OperatorCall::SpanBetween {
                row,
                start,
                end,
                source_len,
            } => crate::domain::span_between::service::between(row, start, end, source_len.0)
                .map(OperatorAnswer::SpanBetween),
        }
    }
}
