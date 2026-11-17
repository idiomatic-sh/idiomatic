//! The `widths` service: free functions named by verb over how many columns the rows of an input of a plan have.

use super::types::InputWidth;
use ryo_relations::domain::relations::service::columns;
use ryo_values::domain::band_joins::types::BandJoin;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// How many columns the rows of `input` have, when the plan says so.
pub(crate) fn width_of(steps: &[PlanStep], input: Input) -> Option<InputWidth> {
    match input {
        Input::BoundObservation(_) => None, // this legacy rewrite does not own the declared bound schema
        Input::Relation(relation) => counted(&columns(relation)),
        Input::Step(id) => match &steps
            .get(usize::try_from(id.0).unwrap_or(usize::MAX))?
            .operation
        {
            Operation::Select { from, kept, .. } => {
                if kept.is_empty() {
                    width_of(steps, *from)
                } else {
                    counted(kept)
                }
            }
            Operation::Join { output, .. } | Operation::BandJoin(BandJoin { output, .. }) => {
                counted(output)
            }
            Operation::Project { columns, .. } => counted(columns),
            Operation::Axis { from, output, .. } => {
                if output.is_empty() {
                    width_of(steps, *from).map(|width| InputWidth(width.0 + 1))
                } else {
                    Some(InputWidth(u32::try_from(output.len()).unwrap_or(u32::MAX)))
                }
            }
            Operation::GroupCount { keys, .. } => {
                counted(keys).map(|width| InputWidth(width.0 + 1))
            }
            Operation::GroupDigest { keys, .. } => {
                counted(keys).map(|width| InputWidth(width.0 + 2))
            }
            Operation::Extend { from, values } => width_of(steps, *from)
                .zip(counted(values))
                .map(|(width, appended)| InputWidth(width.0.saturating_add(appended.0))),
            // A split appends the word to the row it splits.
            Operation::Words { from, .. } => {
                width_of(steps, *from).map(|width| InputWidth(width.0.saturating_add(1)))
            }
            Operation::Union { left, .. }
            | Operation::Intersection { left, .. }
            | Operation::Difference { left, .. }
            | Operation::Antijoin { left, .. } => width_of(steps, *left),
            Operation::Replace { .. }
            | Operation::SpanBounds { .. }
            | Operation::SourceMatches { .. }
            | Operation::RenderSyntax { .. }
            | Operation::FitsRendered { .. }
            | Operation::SpanBetween { .. }
            | Operation::OrderedMatch { .. }
            | Operation::OrderedDfs { .. }
            | Operation::Worklist { .. }
            | Operation::Capture { .. }
            | Operation::Rank { .. }
            | Operation::Closure { .. }
            | Operation::Cycles { .. }
            | Operation::Rows { .. }
            | Operation::Hosted { .. } => None,
        },
    }
}

/// The width of rows of `columns` columns: a grouping answers its keys, then its count, or its digest and its count.
fn counted<T>(columns: &[T]) -> Option<InputWidth> {
    Some(InputWidth(u32::try_from(columns.len()).unwrap_or(u32::MAX)))
}
