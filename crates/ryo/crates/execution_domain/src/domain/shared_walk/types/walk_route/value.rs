//! The route a physical instruction takes relative to the shared source walk.

use crate::domain::step_classes::types::ClassPlace;
use ryo_values::domain::plan_parts::types::Input;

/// Explicit source boundaries remain in request metadata. Adding an operation
/// requires choosing its source-walk barrier in this exhaustive classifier.
#[derive(Clone, Copy, Debug)]
pub enum WalkRoute {
    /// The instruction joins a class of the shared walk.
    Shared(ClassPlace),
    /// The instruction keeps its own walk, for this reason.
    Barrier(WalkBarrier),
}
/// Why an instruction keeps its own source walk.
#[derive(Clone, Copy, Debug)]
pub enum WalkBarrier {
    /// Its representation was declined.
    Representation,
    /// It depends on another instruction's rows.
    DependentInput,
    /// It reads an observation.
    Observation,
    /// It reads a row-varying input.
    RowSemantics,
    /// It reads a derived view or an ordered existence.
    DerivedOrOrdered,
    /// It reads a source the request owns.
    RequestOwnedSource,
    /// It continues another instruction.
    Continuation,
}
/// The barrier that keeps an operation out of the shared walk, if any.
pub fn source_barrier(
    operation: &ryo_values::domain::plans::types::Operation,
    inputs: &[crate::domain::executable_program::types::InputSource],
) -> WalkBarrier {
    use ryo_values::domain::plans::service::{inputs_of, kind_of};
    use ryo_values::domain::query_operators::types::OperationKind;
    // The first input of a selection or a projection is the rows it reads.
    let [first, ..] = inputs_of(operation).list;
    match kind_of(operation) {
        OperationKind::Select | OperationKind::Project => match first {
            Input::Relation(_) => WalkBarrier::Representation,
            // Canonical unary inputs use Step(0) for both ordinary producers
            // and named observations. The resolved boundary retains that distinction.
            Input::Step(_)
                if matches!(
                    inputs.first(),
                    Some(crate::domain::executable_program::types::InputSource::Observation(_))
                ) =>
            {
                WalkBarrier::Observation
            }
            Input::Step(_) => WalkBarrier::DependentInput,
            Input::BoundObservation(_) => WalkBarrier::Observation,
        },
        OperationKind::Extend
        | OperationKind::Capture
        | OperationKind::Rank
        | OperationKind::Replace
        | OperationKind::SpanBetween
        | OperationKind::SourceMatches
        | OperationKind::RenderSyntax
        | OperationKind::FitsRendered
        | OperationKind::SpanBounds => WalkBarrier::RowSemantics,
        OperationKind::Join
        | OperationKind::Union
        | OperationKind::Intersection
        | OperationKind::Difference
        | OperationKind::Antijoin
        | OperationKind::GroupCount
        | OperationKind::GroupDigest
        | OperationKind::OrderedMatch
        | OperationKind::Words
        | OperationKind::Axis
        | OperationKind::Closure
        | OperationKind::OrderedDfs
        | OperationKind::Cycles => WalkBarrier::DerivedOrOrdered,
        OperationKind::Rows | OperationKind::Hosted => WalkBarrier::RequestOwnedSource,
    }
}
