//! The `widths` service of the plan expansion: how many columns the rows of an input of a plan have, from the plan alone. The
//! expansion writes joins and antijoins over every column of an input, so it must count them; the schema algebra
//! (`operation_schema::schema_of`) answers the same number, and the service's cases hold the two together, but it reads column
//! kinds, bindings and the host's declarations, which a plan does not hold.

use ryo_values::domain::plan_parts::types::{Input, RowWidth};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};
use ryo_values::domain::query_plans::types::Plan;

/// How many columns the rows of `input` have among `steps`, the steps of `plan` written so far; none where the plan does not
/// say: the host's rows, whose columns the host declares.
pub(crate) fn width_of(plan: &Plan, steps: &[PlanStep], input: Input) -> Option<RowWidth> {
    match input {
        Input::Relation(relation) => Some(RowWidth(
            ryo_relations::domain::relations::service::kinds(relation).len(),
        )),
        Input::BoundObservation(id) => plan
            .observation_inputs
            .get(usize::from(id))
            .map(|bound| RowWidth(bound.columns.len())),
        Input::Step(id) => {
            let step = steps.get(usize::try_from(id.0).unwrap_or(usize::MAX))?;
            let mut inputs = [None; 3];
            let read = ryo_values::domain::plans::service::inputs_of(&step.operation);
            for (width, read) in inputs
                .iter_mut()
                .zip(ryo_values::domain::plans::service::named(&read))
            {
                *width = width_of(plan, steps, *read);
            }
            width_after(&step.operation, inputs)
        }
    }
}

/// How many columns the rows `operation` answers have, from how many each of its inputs has, in the order it reads them.
pub(crate) fn width_after(
    operation: &Operation,
    inputs: [Option<RowWidth>; 3],
) -> Option<RowWidth> {
    let [first, _, _] = inputs;
    // The width of the first input and so many columns more; none when the plan does not say the input's.
    let more = |columns| first.map(|RowWidth(width)| RowWidth(width + columns));
    match operation {
        Operation::Select { kept, .. } if kept.is_empty() => first,
        Operation::Select { kept, .. } => Some(RowWidth(kept.len())),
        Operation::Project { columns, .. } => Some(RowWidth(columns.len())),
        Operation::Extend { values, .. } => more(values.len()),
        Operation::Join { output, .. }
        | Operation::OrderedMatch { output, .. }
        | Operation::AsOf { output, .. } => Some(RowWidth(output.len())),
        Operation::Axis { output, .. } if output.is_empty() => more(1),
        Operation::Axis { output, .. } => Some(RowWidth(output.len())),
        Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::GroupBound { .. }
        | Operation::FitsRendered { .. } => first,
        Operation::GroupCount { keys, .. } => Some(RowWidth(keys.len() + 1)),
        Operation::GroupDigest { keys, .. }
        | Operation::Closure { keys, .. }
        | Operation::Follow { keys, .. } => Some(RowWidth(keys.len() + 2)),
        Operation::Cycles { .. } => Some(RowWidth(2)),
        Operation::Capture { pattern, .. } => more(
            ryo_operators::domain::capture::service::slot_count(
                ryo_operators::domain::capture::types::SlotPattern(&pattern.0),
            )
            .0,
        ),
        Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::Words { .. }
        | Operation::SpanBetween { .. }
        | Operation::RenderSyntax { .. }
        | Operation::GroupOrdinal { .. } => more(1),
        Operation::SpanBounds { .. } => more(2),
        Operation::SourceMatches {
            base_line,
            capture,
            capture_text,
            capture_line,
            ..
        } => {
            let captured = usize::from(capture.is_some())
                * (1 + usize::from(*capture_text == TextOutput::Appended)
                    + usize::from(*capture_line == LineOutput::Appended));
            more(1 + usize::from(base_line.is_some()) + captured)
        }
        Operation::OrderedDfs { root_keys, .. } => Some(RowWidth(root_keys.len() + 5)),
        Operation::Worklist { root_keys, .. } => Some(RowWidth(root_keys.len() + 6)),
        Operation::Rows { keys, .. } => Some(RowWidth(keys.len())),
        Operation::Hosted { .. } => None,
    }
}
