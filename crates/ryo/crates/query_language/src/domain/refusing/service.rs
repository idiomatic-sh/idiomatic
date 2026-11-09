//! The `refusing` service: the steps of a statement just lowered, refused when they cannot run where they are placed.

use ryo_relations::domain::relations::types::CellKind;

use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{Builder, FileOrigin, ScopedInput};
use crate::domain::planning::service as planning;
use crate::domain::resolving::service::column_of;
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;

/// Refuses a step from `from` on of the plan under construction that compares rows on spans alone, with no file or node key,
/// over rows made after the files: a span states no file, so equal spans of two files would meet, merge or cancel. Such a step is
/// kept only when every input is per-file, so it runs inside one file. This covers a join, an antijoin, an ordered match, a group
/// count or digest, a closure, an ordered depth-first visit, the cycles of a graph, a union, an intersection, a difference, and
/// the distinct rows of a projection whose rows hold a span and no file or node.
pub(crate) fn refuse_spans_alone(
    builder: &Builder<'_>,
    from: parts::StepId,
) -> Result<(), LoweringError> {
    let first = usize::try_from(from.0).unwrap_or(usize::MAX);
    for (step, lowered) in builder.steps.iter().zip(&builder.lowered).skip(first) {
        let inputs: Vec<ScopedInput> = planning::inputs_of(&step.operation)
            .into_iter()
            .map(|input| planning::scoped(builder, input))
            .collect();
        if !inputs
            .iter()
            .any(|input| input.scope == ryo_values::domain::plan_parts::types::StepScope::CrossFile)
        {
            continue;
        }
        let first = inputs
            .first()
            .map_or(&[][..], |input| input.fields.as_slice());
        let second = inputs
            .get(1)
            .map_or(&[][..], |input| input.fields.as_slice());
        let on_spans_alone = |held: fn(CellKind) -> FileOrigin| match &step.operation {
            // The distinct rows compare every column they keep, whatever their input held.
            plans::Operation::Project {
                distinct: parts::RepeatedRows::Dropped,
                ..
            } => planning::keyed(&lowered.fields, held),
            plans::Operation::Join { .. }
            | plans::Operation::Antijoin { .. }
            | plans::Operation::OrderedMatch { .. }
            | plans::Operation::GroupCount { .. }
            | plans::Operation::GroupDigest { .. }
            | plans::Operation::Closure { .. }
            | plans::Operation::OrderedDfs { .. }
            | plans::Operation::Cycles { .. }
            | plans::Operation::Union { .. }
            | plans::Operation::Intersection { .. }
            | plans::Operation::Difference { .. } => {
                planning::placed(&step.operation, first, second, &lowered.fields, held)
            }
            plans::Operation::Project {
                distinct: parts::RepeatedRows::Kept,
                ..
            }
            | plans::Operation::SpanBetween { .. }
            | plans::Operation::SpanBounds { .. }
            | plans::Operation::SourceMatches { .. }
            | plans::Operation::RenderSyntax { .. }
            | plans::Operation::FitsRendered { .. }
            | plans::Operation::Replace { .. }
            | plans::Operation::Select { .. }
            | plans::Operation::Extend { .. }
            | plans::Operation::Capture { .. }
            | plans::Operation::Rank { .. }
            | plans::Operation::Words { .. }
            | plans::Operation::Axis { .. }
            | plans::Operation::Rows { .. }
            | plans::Operation::Hosted { .. } => FileOrigin::Loose,
        };
        if on_spans_alone(planning::held_in_file) == FileOrigin::Loose
            && on_spans_alone(planning::held_inside_one_file) == FileOrigin::Held
        {
            return Err(LoweringError::CrossFileSpanKey { span: builder.span });
        }
    }
    Ok(())
}

/// Refuses a closure or cycles step from `from` on of the plan under construction whose source and destination are of two kinds:
/// a walk goes from a source to a destination that is the next source, so both are of one kind, and where the step runs is decided
/// on its source alone.
pub(crate) fn refuse_mixed_vertices(
    builder: &Builder<'_>,
    from: parts::StepId,
) -> Result<(), LoweringError> {
    let first = usize::try_from(from.0).unwrap_or(usize::MAX);
    for step in builder.steps.iter().skip(first) {
        let (plans::Operation::Closure {
            edges,
            source,
            destination,
            ..
        }
        | plans::Operation::Cycles {
            edges,
            source,
            destination,
        }) = &step.operation
        else {
            continue;
        };
        let fields = planning::scoped(builder, *edges).fields;
        if column_of(&fields, *source).kind != column_of(&fields, *destination).kind {
            return Err(LoweringError::MixedVertices { span: builder.span });
        }
    }
    Ok(())
}
