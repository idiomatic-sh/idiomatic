//! Historical fragment counts and output for parity evidence only.
use super::constants::COUNTING_OF_STEP;
use super::service::make;
use super::types::FragmentRun;
use crate::domain::lowering::service::{shape_at, step_of};
use crate::domain::lowering::types::Counting;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, OperatorContext};
use crate::domain::operator::contract::types::PrimitiveSemantics;
use crate::domain::positions::service::{column_at, count, key_into, row, slot_of, value, values};
use crate::domain::positions::types::LoweredRoom;
use crate::domain::table_model::service::positions;
use ryo_relations::domain::relations::types::{Integer, Value};
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Operation;

/// Whether the step `id` counts the rows of a lowered step.
///
/// # Panics
///
/// On a step the plan does not have: the steps come only from the plan that was lowered, so another is a bug.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "a step comes only from the plan that was lowered; another is a bug that must crash, not a step read as counting nothing"
)]
pub(crate) fn counts(run: &FragmentRun<'_, '_>, id: StepId) -> Counting {
    run.lowering
        .counting
        .get(usize::try_from(id.0).unwrap_or(usize::MAX))
        .copied()
        .expect(COUNTING_OF_STEP)
}

/// Counts the rows of the lowered step the step `id` groups, and passes the counts on: for each group of the keys, in the order
/// the groups first appear, the keys and how many rows or how many different values of the column counted.
///
/// # Errors
///
/// What the lowered step or the one that takes the rows refuses.
pub(crate) fn emit_counts(
    semantics: &dyn PrimitiveSemantics,
    run: &mut FragmentRun<'_, '_>,
    id: StepId,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let Operation::GroupCount {
        from,
        keys,
        distinct,
    } = &run
        .plan
        .steps
        .get(usize::try_from(id.0).unwrap_or(usize::MAX))
        .ok_or(OperatingError::NotRun)?
        .operation
    else {
        return Err(OperatingError::NotRun);
    };
    let input = step_of(*from)?;
    make(semantics, run, input, context)?;
    let shape = shape_at(run.lowering, input)?;
    let tree = run.tree;
    let LoweredRoom {
        made,
        keys: flat,
        cells,
        ..
    } = &mut *run.room;
    let rows = made
        .get(usize::try_from(input.0).unwrap_or(usize::MAX))
        .ok_or(OperatingError::NotRun)?;
    let stride = keys.len() + usize::from(distinct.is_some());
    flat.0.clear();
    for at in positions(count(rows)) {
        let each = row(rows, at);
        key_into(&tree, &shape.columns, each, keys, flat);
        if let Some(column) = distinct {
            key_into(
                &tree,
                &shape.columns,
                each,
                std::slice::from_ref(column),
                flat,
            );
        }
    }
    run.derived
        .with_raw_order(&flat.0, stride.max(1), &mut |order| {
            let (flat, order) = (&flat.0, order.rows());
            let wide = keys.len();
            // Each group: where it first appears among the rows, and how many it counts.
            let mut groups: Vec<(usize, usize)> = Vec::new();
            // The key of a row among the flat keys, and the distinct value after it.
            let key_at = |row| flat.get(row * stride..row * stride + wide);
            let distinct_at = |row| flat.get(row * stride + wide);
            let mut from_place = 0;
            while let Some(&head) = order.get(from_place) {
                let mut to_place = from_place;
                let mut first = head;
                let mut counted = 0;
                let mut last_distinct: Option<usize> = None;
                while let Some(&each) = order.get(to_place) {
                    if key_at(each) != key_at(head) {
                        break;
                    }
                    first = first.min(each);
                    if distinct.is_none() {
                        counted += 1;
                    } else {
                        let same = last_distinct
                            .is_some_and(|before| distinct_at(before) == distinct_at(each));
                        if !same {
                            counted += 1;
                            last_distinct = Some(each);
                        }
                    }
                    to_place += 1;
                }
                groups.push((first, counted));
                from_place = to_place;
            }
            groups.sort_unstable();
            for (first, counted) in groups {
                cells.clear();
                let each = row(rows, ryo_values::domain::rows::types::RowIndex(first));
                for index in keys {
                    let column = column_at(&shape.columns, *index);
                    cells.push(value(&tree, column.field, slot_of(each, column)));
                }
                cells.push(Value::Number(Integer(
                    i64::try_from(counted).unwrap_or(i64::MAX),
                )));
                emit.row(context, cells)?;
            }
            Ok(())
        })
}

/// Passes on the rows of the step `id`, as values.
///
/// # Errors
///
/// What the one that takes the rows refuses.
pub(crate) fn emit(
    run: &mut FragmentRun<'_, '_>,
    id: StepId,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let shape = shape_at(run.lowering, id)?;
    let tree = run.tree;
    let LoweredRoom { made, cells, .. } = &mut *run.room;
    let rows = made
        .get(usize::try_from(id.0).unwrap_or(usize::MAX))
        .ok_or(OperatingError::NotRun)?;
    for at in positions(count(rows)) {
        values(&tree, &shape.columns, row(rows, at), cells);
        emit.row(context, cells)?;
    }
    Ok(())
}
