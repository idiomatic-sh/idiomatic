//! The `lineage` service: free functions named by verb over the relation column a column of a step's rows was read from, and the
//! per-file steps its values come from.

use super::types::LineageMemo;
use super::types::StepColumn;
use super::types::ValueOrigin;
use crate::domain::fusing::service::source_column;
use crate::domain::planning::service::planned_scope;
use crate::domain::resolving::service::relation_fields;
use crate::domain::walking::service::width_of;
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::lineage::service::anchor_of;
use ryo_values::domain::lineage::service::one_file;
use ryo_values::domain::lineage::types::ColumnAnchor;
use ryo_values::domain::lineage::types::Lineage;
use ryo_values::domain::plan_parts::service::column_in;
use ryo_values::domain::plan_parts::service::step_in;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// The kind of the column `column` of the rows of `input`, when it was read from a relation's column, through steps that keep, filter,
/// join, combine or group rows: none for a column a step made (a count, a capture, an appended value), or one the plan cannot follow.
/// A step that appends columns to the rows it reads is followed into what it reads; a column past those rows is past every column
/// there as well, and so is none.
#[must_use]
pub(crate) fn column_kind(
    steps: &[PlanStep],
    memo: &mut LineageMemo,
    input: Input,
    column: ColumnIndex,
) -> Option<CellKind> {
    let id = match input {
        Input::Relation(relation) => {
            return column_in(&relation_fields(relation), column).map(|field| field.kind);
        }
        Input::BoundObservation(_) => return None,
        Input::Step(id) => id,
    };
    if let Some(found) = memo.kinds.get(&StepColumn { step: id, column }) {
        return *found;
    }
    let found = step_kind(steps, memo, id, column);
    let _first = memo.kinds.insert(StepColumn { step: id, column }, found);
    found
}

/// The kind of the column `column` of the step `id`, followed into what the step reads.
fn step_kind(
    steps: &[PlanStep],
    memo: &mut LineageMemo,
    id: StepId,
    column: ColumnIndex,
) -> Option<CellKind> {
    match &step_in(steps, id)?.operation {
        Operation::Select { from, kept, .. } => {
            column_kind(steps, memo, *from, source_column(kept, column)?)
        }
        Operation::Project { from, columns, .. } => {
            column_kind(steps, memo, *from, *column_in(columns, column)?)
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
                Side::Left => column_kind(steps, memo, *left, at),
                Side::Right => column_kind(steps, memo, *right, source_column(view, at)?),
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
                Side::Left => column_kind(steps, memo, *left, at),
                Side::Right => column_kind(steps, memo, *right, at),
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
                return column_kind(steps, memo, *from, column);
            }
            let JoinColumn { side, column: at } = *column_in(output, column)?;
            match side {
                Side::Left => column_kind(steps, memo, *from, at),
                Side::Right => column_kind(steps, memo, *domain, source_column(domain_view, at)?),
            }
        }
        Operation::GroupCount { from, keys, .. } | Operation::GroupDigest { from, keys, .. } => {
            column_kind(steps, memo, *from, *column_in(keys, column)?)
        }
        Operation::Union { left, .. }
        | Operation::Intersection { left, .. }
        | Operation::Difference { left, .. }
        | Operation::Antijoin { left, .. } => column_kind(steps, memo, *left, column),
        Operation::Extend { from, .. }
        | Operation::Capture { from, .. }
        | Operation::Rank { from, .. }
        | Operation::Replace { from, .. }
        | Operation::Words { from, .. }
        | Operation::SpanBetween { from, .. }
        | Operation::SourceMatches { from, .. }
        | Operation::RenderSyntax { from, .. }
        | Operation::FitsRendered { from, .. }
        | Operation::SpanBounds { from, .. } => column_kind(steps, memo, *from, column),
        Operation::OrderedDfs { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => None,
    }
}

/// The columns of per-file steps whose values the column `column` of the rows of `input`, a step after the files, holds: every value of
/// it is a value of one of them, since a step after the files only keeps, filters, joins, combines or groups the rows the files hand
/// over. Through a join, a key the per-file side cannot be followed into is followed into the key it equals; a union holds the values
/// of both its inputs. None when the plan cannot follow the column to per-file steps, or it comes from a whole relation, which holds
/// every value of a file.
#[must_use]
pub(crate) fn value_origins(
    steps: &[PlanStep],
    memo: &mut LineageMemo,
    input: Input,
    column: ColumnIndex,
) -> Option<Vec<ValueOrigin>> {
    let Input::Step(id) = input else {
        return None;
    };
    if let Some(found) = memo.origins.get(&StepColumn { step: id, column }) {
        return found.clone();
    }
    let found = step_origins(steps, memo, id, column);
    let _first = memo.origins.insert(StepColumn { step: id, column }, found.clone());
    found
}

/// The per-file origins of the column `column` of the step `id`, followed into what the step reads.
fn step_origins(
    steps: &[PlanStep],
    memo: &mut LineageMemo,
    id: StepId,
    column: ColumnIndex,
) -> Option<Vec<ValueOrigin>> {
    let input = Input::Step(id);
    if planned_scope(steps, input)? == StepScope::PerFile {
        return Some(vec![ValueOrigin { input, column }]);
    }
    match &step_in(steps, id)?.operation {
        Operation::Select { from, kept, .. } => {
            value_origins(steps, memo, *from, source_column(kept, column)?)
        }
        Operation::Project { from, columns, .. } => {
            value_origins(steps, memo, *from, *column_in(columns, column)?)
        }
        Operation::Join {
            left,
            right,
            left_on,
            right_on,
            output,
            view,
            ..
        } => {
            let JoinColumn { side, column: at } = *column_in(output, column)?;
            let (own, own_column, other, own_keys, other_keys) = match side {
                Side::Left => (*left, at, *right, left_on, right_on),
                Side::Right => (*right, source_column(view, at)?, *left, right_on, left_on),
            };
            value_origins(steps, memo, own, own_column).or_else(|| {
                let key = own_keys.iter().position(|each| *each == at)?;
                let equal = *other_keys.get(key)?;
                match side {
                    Side::Left => value_origins(steps, memo, other, source_column(view, equal)?),
                    Side::Right => value_origins(steps, memo, other, equal),
                }
            })
        }
        Operation::Union { left, right } => {
            let mut origins = value_origins(steps, memo, *left, column)?;
            for origin in value_origins(steps, memo, *right, column)? {
                if !origins.contains(&origin) {
                    origins.push(origin);
                }
            }
            Some(origins)
        }
        Operation::Intersection { left, .. }
        | Operation::Difference { left, .. }
        | Operation::Antijoin { left, .. } => value_origins(steps, memo, *left, column),
        Operation::GroupCount { from, keys, .. } | Operation::GroupDigest { from, keys, .. } => {
            value_origins(steps, memo, *from, *column_in(keys, column)?)
        }
        Operation::Extend { from, .. }
        | Operation::Capture { from, .. }
        | Operation::Rank { from, .. }
        | Operation::Replace { from, .. }
        | Operation::Words { from, .. }
        | Operation::SpanBetween { from, .. }
        | Operation::SourceMatches { from, .. }
        | Operation::RenderSyntax { from, .. }
        | Operation::FitsRendered { from, .. }
        | Operation::SpanBounds { from, .. } => {
            // A column these append is made after the files, and is followed no further.
            (column.0 < width_of(steps, *from)?.0)
                .then(|| value_origins(steps, memo, *from, column))
                .flatten()
        }
        Operation::OrderedMatch { .. }
        | Operation::Axis { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => None,
    }
}

/// The lineage of the rows of `input` by the one rule of `ryo_values`' lineage, when each comes from one file: a relation's, whose rows
/// are partitioned by file, or a per-file step's, whose columns state their row's file when they are files or nodes. A column whose kind
/// the plan cannot follow states nothing, and the rows of a step after the files or of a bound input are mixed: a rewrite that needs a
/// proof that two rows come from one file reads it here and nowhere else.
pub(crate) fn file_lineage(steps: &[PlanStep], memo: &mut LineageMemo, input: Input) -> Lineage {
    let id = match input {
        Input::Relation(relation) => return one_file(kinds(relation)),
        Input::BoundObservation(_) => return Lineage::Mixed,
        Input::Step(id) => id,
    };
    if let Some(found) = memo.lineages.get(&id) {
        return found.clone();
    }
    let found = match (planned_scope(steps, input), width_of(steps, input)) {
        (Some(StepScope::PerFile), Some(width)) => Lineage::OneFile(
            (0..width.0)
                .map(|column| {
                    column_kind(steps, memo, input, ColumnIndex(column))
                        .map_or(ColumnAnchor::Elsewhere, anchor_of)
                })
                .collect(),
        ),
        (Some(StepScope::CrossFile) | None, _) | (Some(StepScope::PerFile), None) => Lineage::Mixed,
    };
    let _first = memo.lineages.insert(id, found.clone());
    found
}
