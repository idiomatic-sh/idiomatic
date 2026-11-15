//! The `distinctness` service: free functions named by verb over whether the rows of a step hold each value of a set of its columns
//! once, followed from a relation's key column through the steps that keep it.

use super::types::DistinctMemo;
use super::types::StepKeys;
use crate::domain::lowering::types::Uniqueness;
use crate::domain::resolving::service::relation_fields;
use ryo_values::domain::plan_parts::service::column_in;
use ryo_values::domain::plan_parts::service::step_in;
use ryo_values::domain::plan_parts::service::view_source;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// Whether no two rows of `input` hold the same values in the columns `keys`: so that a join keyed on them pairs each row of its other
/// input with one row of `input` at most, and answers no more rows than that input has. Proven of a relation's key column, of a
/// projection or a grouping keeping no column outside `keys` (its rows are distinct), and through steps that filter rows, keep them
/// as they are, or join them to one row at most of their other input; any other step, or a column the plan cannot follow, is
/// `Shared`.
#[must_use]
pub(crate) fn distinct_on(
    steps: &[PlanStep],
    memo: &mut DistinctMemo,
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
    if let Some(found) = memo.0.get(&at) {
        return *found;
    }
    let found = step_distinct(steps, memo, id, &at.columns).unwrap_or(Uniqueness::Shared);
    let _first = memo.0.insert(at, found);
    found
}

/// Whether the rows of the step `id` are distinct on `keys`, followed into what the step reads: none when the plan cannot follow them.
fn step_distinct(
    steps: &[PlanStep],
    memo: &mut DistinctMemo,
    id: StepId,
    keys: &[ColumnIndex],
) -> Option<Uniqueness> {
    Some(match &step_in(steps, id)?.operation {
        Operation::Select { from, kept, .. } => {
            let keys = keys
                .iter()
                .map(|key| view_source(kept, *key))
                .collect::<Option<Vec<_>>>()?;
            distinct_on(steps, memo, *from, &keys)
        }
        Operation::Project { from, columns, .. } => {
            if covers(keys, columns) == Uniqueness::Unique {
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
            if covers(keys, grouped) == Uniqueness::Unique {
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
            let one_each = |memo: &mut DistinctMemo,
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

/// Whether `keys` holds every column of a step that answers one row for each set of values of the columns `made` names: then its
/// rows are distinct on `keys`.
fn covers<T>(keys: &[ColumnIndex], made: &[T]) -> Uniqueness {
    if (0..made.len())
        .all(|column| u32::try_from(column).is_ok_and(|column| keys.contains(&ColumnIndex(column))))
    {
        Uniqueness::Unique
    } else {
        Uniqueness::Shared
    }
}
