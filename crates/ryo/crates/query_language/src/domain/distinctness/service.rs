//! The `distinctness` service: free functions named by verb over whether the rows of a step hold each value of a set of its columns
//! once, followed from a relation's key column through the steps that keep it.

use super::types::DistinctMemo;
use super::types::FileKeyed;
use super::types::StepKeys;
use crate::domain::lineage::service::column_kind;
use crate::domain::lineage::types::LineageMemo;
use crate::domain::lowering::types::Uniqueness;
use crate::domain::resolving::service::relation_fields;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::service::column_in;
use ryo_values::domain::plan_parts::service::step_in;
use ryo_values::domain::plan_parts::service::view_source;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::KeyFan;
use ryo_values::domain::plan_parts::types::RepeatedRows;
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
        // A projection that drops repeated rows is distinct on every column it keeps; one that keeps them (`Kept`, chosen because
        // a column kept was said to tell its rows apart) is distinct only on what the rows it reads are distinct on.
        Operation::Project {
            from,
            columns,
            distinct,
        } => {
            if *distinct == RepeatedRows::Dropped && covers(keys, columns) == Uniqueness::Unique {
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
        | Operation::BandJoin { .. }
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
        | Operation::Worklist { .. }
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

/// Whether the keys `keys` of `input` are the file alone: the one definition the planner's filter placement and a join's proven
/// fan read.
pub(crate) fn keyed_on_file_alone(
    steps: &[PlanStep],
    memo: &mut LineageMemo,
    input: Input,
    keys: &[ColumnIndex],
) -> FileKeyed {
    if !keys.is_empty()
        && keys
            .iter()
            .all(|key| column_kind(steps, memo, input, *key) == Some(CellKind::File))
    {
        FileKeyed::Alone
    } else {
        FileKeyed::Not
    }
}

/// What a join's keys are proven to pair: as written for an `all-pairs join`; `Unique` when either side is distinct on its keys
/// (the right through its view); `FileOnly` when the left keys are the file alone; `Shared` otherwise.
pub(crate) fn key_fan(
    steps: &[PlanStep],
    memos: (&mut DistinctMemo, &mut LineageMemo),
    sides: [(Input, &[ColumnIndex]); 2],
    view: &[ColumnIndex],
    written: KeyFan,
) -> KeyFan {
    let (distinct, kinds) = memos;
    let [(left, left_on), (right, right_on)] = sides;
    if written == KeyFan::AllPairs {
        return KeyFan::AllPairs;
    }
    let right_keys: Option<Vec<ColumnIndex>> =
        right_on.iter().map(|key| view_source(view, *key)).collect();
    if right_keys.is_some_and(|keys| {
        !keys.is_empty() && distinct_on(steps, distinct, right, &keys) == Uniqueness::Unique
    }) || (!left_on.is_empty()
        && distinct_on(steps, distinct, left, left_on) == Uniqueness::Unique)
    {
        KeyFan::Unique
    } else if keyed_on_file_alone(steps, kinds, left, left_on) == FileKeyed::Alone {
        KeyFan::FileOnly
    } else {
        KeyFan::Shared
    }
}

/// The plan with each join's fan proven by `key_fan`: run once the plan's steps are placed, before it crosses to the operators.
#[must_use]
pub(crate) fn fanned(mut steps: Vec<PlanStep>) -> Vec<PlanStep> {
    let mut distinct = DistinctMemo::default();
    let mut kinds = LineageMemo::default();
    let proven: Vec<Option<KeyFan>> = steps
        .iter()
        .map(|step| match &step.operation {
            Operation::Join {
                left,
                right,
                left_on,
                right_on,
                view,
                fan,
                reach: None,
                ..
            }
            | Operation::BandJoin {
                left,
                right,
                left_on,
                right_on,
                view,
                fan,
                ..
            } => Some(key_fan(
                &steps,
                (&mut distinct, &mut kinds),
                [(*left, left_on), (*right, right_on)],
                view,
                *fan,
            )),
            _ => None,
        })
        .collect();
    for (step, found) in steps.iter_mut().zip(proven) {
        if let (Operation::Join { fan, .. } | Operation::BandJoin { fan, .. }, Some(found)) =
            (&mut step.operation, found)
        {
            *fan = found;
        }
    }
    steps
}
