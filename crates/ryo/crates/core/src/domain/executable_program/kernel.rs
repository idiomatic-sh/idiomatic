//! Semantic kernels have only admitted borrowed inputs, views and scratch.
//! They cannot ask an engine to resolve a dependency or mint a publication.
use crate::domain::{
    operating::errors::OperatingError,
    operator::contract::types::{PreparedInputs, PrimitiveLease},
};
use ryo_values::domain::plans::types::{Operation, OperationKind};

/// The complete family is deliberately matched here. Adding a primitive must
/// extend both registration and this required semantic boundary.
pub(crate) fn evaluate(
    kind: OperationKind,
    operation: &Operation,
    inputs: PreparedInputs<'_>,
    lease: &mut PrimitiveLease<'_>,
    output: &mut dyn crate::domain::operator::contract::types::RowSink,
) -> Result<(), OperatingError> {
    match kind {
        OperationKind::Select
        | OperationKind::Project
        | OperationKind::Extend
        | OperationKind::Join
        | OperationKind::Union
        | OperationKind::Intersection
        | OperationKind::Difference
        | OperationKind::Antijoin
        | OperationKind::GroupCount
        | OperationKind::GroupDigest
        | OperationKind::Capture
        | OperationKind::Rank
        | OperationKind::Words
        | OperationKind::Replace
        | OperationKind::OrderedMatch
        | OperationKind::Axis
        | OperationKind::Closure
        | OperationKind::Cycles
        | OperationKind::OrderedDfs
        | OperationKind::Rows
        | OperationKind::Hosted
        | OperationKind::SpanBetween
        | OperationKind::SourceMatches
        | OperationKind::RenderSyntax
        | OperationKind::FitsRendered
        | OperationKind::SpanBounds => {
            if crate::domain::features::service::kind_of(operation) != kind {
                return Err(OperatingError::WrongOperation);
            }
            crate::domain::operator::service::registered(kind)
                .evaluate(operation, inputs, lease, output)
        }
    }
}
