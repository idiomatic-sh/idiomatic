//! The route a physical instruction takes relative to the shared source walk.

use crate::domain::positions::types::ClassPlace;
use ryo_values::domain::plan_parts::types::Input;

/// Explicit source boundaries remain in request metadata. Adding an operation
/// requires choosing its source-walk barrier in this exhaustive classifier.
#[derive(Clone, Copy, Debug)]
pub(crate) enum WalkRoute {
    Shared(ClassPlace),
    Barrier(WalkBarrier),
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum WalkBarrier {
    Representation,
    DependentInput,
    Observation,
    RowSemantics,
    DerivedOrOrdered,
    RequestOwnedSource,
    Continuation,
}
pub(crate) fn source_barrier(
    operation: &ryo_values::domain::plans::types::Operation,
    inputs: &[crate::domain::executable_program::types::InputSource],
) -> WalkBarrier {
    use ryo_values::domain::plans::types::Operation;
    match operation {
        Operation::Select { from, .. } | Operation::Project { from, .. } => match from {
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
        Operation::Extend { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. } => WalkBarrier::RowSemantics,
        Operation::Join { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Words { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Cycles { .. } => WalkBarrier::DerivedOrOrdered,
        Operation::Rows { .. } | Operation::Hosted { .. } => WalkBarrier::RequestOwnedSource,
    }
}
