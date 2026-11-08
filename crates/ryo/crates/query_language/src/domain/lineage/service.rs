//! The `lineage` service: free functions named by verb over the relation column a column of a step's rows was read from.

use crate::domain::fusing::service::source_column;
use crate::domain::resolving::service::relation_fields;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::service::column_in;
use ryo_values::domain::plan_parts::service::step_in;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// The kind of the column `column` of the rows of `input`, when it was read from a relation's column, through steps that keep, filter,
/// join, combine or group rows: none for a column a step made (a count, a capture, an appended value), or one the plan cannot follow.
/// A step that appends columns to the rows it reads is followed into what it reads; a column past those rows is past every column
/// there as well, and so is none.
#[must_use]
pub(crate) fn column_kind(steps: &[PlanStep], input: Input, column: ColumnIndex) -> Option<CellKind> {
    let id = match input {
        Input::Relation(relation) => {
            return column_in(&relation_fields(relation), column).map(|field| field.kind);
        }
        Input::BoundObservation(_) => return None,
        Input::Step(id) => id,
    };
    match &step_in(steps, id)?.operation {
        Operation::Select { from, kept, .. } => {
            column_kind(steps, *from, source_column(kept, column)?)
        }
        Operation::Project { from, columns, .. } => {
            column_kind(steps, *from, *column_in(columns, column)?)
        }
        Operation::Join {
            left,
            right,
            output,
            view,
            ..
        } => {
            let JoinColumn { side, column: at } = *column_in(output, column)?;
            match side {
                Side::Left => column_kind(steps, *left, at),
                Side::Right => column_kind(steps, *right, source_column(view, at)?),
            }
        }
        Operation::OrderedMatch {
            left,
            right,
            output,
            ..
        } => {
            let JoinColumn { side, column: at } = *column_in(output, column)?;
            match side {
                Side::Left => column_kind(steps, *left, at),
                Side::Right => column_kind(steps, *right, at),
            }
        }
        Operation::Axis {
            from,
            domain,
            output,
            domain_view,
            ..
        } => {
            if output.is_empty() {
                return column_kind(steps, *from, column);
            }
            let JoinColumn { side, column: at } = *column_in(output, column)?;
            match side {
                Side::Left => column_kind(steps, *from, at),
                Side::Right => column_kind(steps, *domain, source_column(domain_view, at)?),
            }
        }
        Operation::GroupCount { from, keys, .. } | Operation::GroupDigest { from, keys, .. } => {
            column_kind(steps, *from, *column_in(keys, column)?)
        }
        Operation::Union { left, .. }
        | Operation::Intersection { left, .. }
        | Operation::Difference { left, .. }
        | Operation::Antijoin { left, .. } => column_kind(steps, *left, column),
        Operation::Extend { from, .. }
        | Operation::Capture { from, .. }
        | Operation::Rank { from, .. }
        | Operation::Replace { from, .. }
        | Operation::Words { from, .. }
        | Operation::SpanBetween { from, .. }
        | Operation::SourceMatches { from, .. }
        | Operation::RenderSyntax { from, .. }
        | Operation::FitsRendered { from, .. }
        | Operation::SpanBounds { from, .. } => column_kind(steps, *from, column),
        Operation::OrderedDfs { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => None,
    }
}
