//! The `lineage` service: free functions named by verb over the relation column a column of a step's rows was read from, and the
//! per-file steps its values come from.

use super::types::LineageMemo;
use super::types::StepColumn;
use super::types::StepKeys;
use super::types::ValueOrigin;
use crate::domain::lowering::types::Uniqueness;
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
use ryo_values::domain::plan_parts::service::view_source;
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
            column_kind(steps, memo, *from, view_source(kept, column)?)
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
                Side::Right => column_kind(steps, memo, *right, view_source(view, at)?),
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
                Side::Right => column_kind(steps, memo, *domain, view_source(domain_view, at)?),
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

/// Whether no two rows of `input` hold the same values in the columns `keys`: so that a join keyed on them pairs each row of its other
/// input with one row of `input` at most, and answers no more rows than that input has. Proven of a relation's key column, of a
/// projection or a grouping keeping no column outside `keys` (its rows are distinct), and through steps that filter rows, keep them
/// as they are, or join them to one row at most of their other input; any other step, or a column the plan cannot follow, is
/// `Shared`.
#[must_use]
pub(crate) fn distinct_on(
    steps: &[PlanStep],
    memo: &mut LineageMemo,
    input: Input,
    keys: &[ColumnIndex],
) -> Uniqueness {
    let id = match input {
        Input::Relation(relation) => {
            let fields = relation_fields(relation);
            return if keys.iter().any(|key| {
                column_in(&fields, *key).is_some_and(|field| field.unique == Uniqueness::Unique)
            }) {
                Uniqueness::Unique
            } else {
                Uniqueness::Shared
            };
        }
        Input::BoundObservation(_) => return Uniqueness::Shared,
        Input::Step(id) => id,
    };
    let mut columns = keys.to_vec();
    columns.sort_unstable();
    columns.dedup();
    let at = StepKeys { step: id, columns };
    if let Some(found) = memo.distinct.get(&at) {
        return *found;
    }
    let found = step_distinct(steps, memo, id, &at.columns).unwrap_or(Uniqueness::Shared);
    let _first = memo.distinct.insert(at, found);
    found
}

/// Whether the rows of the step `id` are distinct on `keys`, followed into what the step reads: none when the plan cannot follow them.
fn step_distinct(
    steps: &[PlanStep],
    memo: &mut LineageMemo,
    id: StepId,
    keys: &[ColumnIndex],
) -> Option<Uniqueness> {
    // Every one of the first `width` columns is a key: the rows the step makes distinct are distinct on `keys`.
    let covers = |width: usize| {
        (0..width).all(|column| {
            keys.iter()
                .any(|key| usize::try_from(key.0).is_ok_and(|key| key == column))
        })
    };
    Some(match &step_in(steps, id)?.operation {
        Operation::Select { from, kept, .. } => {
            let keys = keys
                .iter()
                .map(|key| view_source(kept, *key))
                .collect::<Option<Vec<_>>>()?;
            distinct_on(steps, memo, *from, &keys)
        }
        Operation::Project { from, columns, .. } => {
            if covers(columns.len()) {
                Uniqueness::Unique
            } else {
                let keys = keys
                    .iter()
                    .map(|key| column_in(columns, *key).copied())
                    .collect::<Option<Vec<_>>>()?;
                distinct_on(steps, memo, *from, &keys)
            }
        }
        Operation::GroupCount { keys: grouped, .. }
        | Operation::GroupDigest { keys: grouped, .. } => {
            if covers(grouped.len()) {
                Uniqueness::Unique
            } else {
                Uniqueness::Shared
            }
        }
        Operation::Join {
            left,
            right,
            left_on,
            right_on,
            output,
            view,
            reach: None,
            ..
        } => {
            let mut left_keys = Vec::new();
            let mut right_keys = Vec::new();
            for key in keys {
                let JoinColumn { side, column } = *column_in(output, *key)?;
                match side {
                    Side::Left => left_keys.push(column),
                    Side::Right => right_keys.push(view_source(view, column)?),
                }
            }
            let right_on = right_on
                .iter()
                .map(|key| view_source(view, *key))
                .collect::<Option<Vec<_>>>()?;
            // A row of one input distinct on its keys, met by one row at most of the other, makes one row of the answer.
            let one_each = |memo: &mut LineageMemo,
                            own: Input,
                            own_keys: &[ColumnIndex],
                            other: Input,
                            other_on: &[ColumnIndex]| {
                !own_keys.is_empty()
                    && distinct_on(steps, memo, own, own_keys) == Uniqueness::Unique
                    && distinct_on(steps, memo, other, other_on) == Uniqueness::Unique
            };
            if one_each(memo, *left, &left_keys, *right, &right_on)
                || one_each(memo, *right, &right_keys, *left, left_on)
            {
                Uniqueness::Unique
            } else {
                Uniqueness::Shared
            }
        }
        Operation::Intersection { left, .. }
        | Operation::Difference { left, .. }
        | Operation::Antijoin { left, .. } => distinct_on(steps, memo, *left, keys),
        Operation::Join { .. }
        | Operation::Union { .. }
        | Operation::Extend { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::Words { .. }
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Axis { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => Uniqueness::Shared,
    })
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
    let _first = memo
        .origins
        .insert(StepColumn { step: id, column }, found.clone());
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
            value_origins(steps, memo, *from, view_source(kept, column)?)
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
                Side::Right => (*right, view_source(view, at)?, *left, right_on, left_on),
            };
            value_origins(steps, memo, own, own_column).or_else(|| {
                let key = own_keys.iter().position(|each| *each == at)?;
                let equal = *other_keys.get(key)?;
                match side {
                    Side::Left => value_origins(steps, memo, other, view_source(view, equal)?),
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
