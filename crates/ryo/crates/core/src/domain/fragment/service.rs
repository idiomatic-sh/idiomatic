//! The `fragment` service: the lowered steps of a plan run over the positions of the elements of a file, each step made once, a value
//! made only where a comparison, a key or the step that takes the rows asks for one.

use super::constants::COUNTING_OF_STEP;
use ryo_language::domain::relations::types::Integer;
use ryo_language::domain::relations::types::Value;

use super::types::FragmentRun;
use crate::domain::joins::service::{antijoin, join, union};
use crate::domain::lowering::service::{shape_at, step_of};
use crate::domain::lowering::types::Counting;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, OperatorContext};
use crate::domain::positions::service::{
    column_at, count, key_into, row, selected, slot_of, sorted, value, values,
};
use crate::domain::positions::types::{
    KeyColumns, LoweredRoom, Positions, Readiness, Sides, Slots, Stride, Work,
};
use crate::domain::selections::service::{project, select_rows, select_tree};
use crate::domain::selections::types::ProjectedColumns;
use crate::domain::table_model::service::positions;
use crate::domain::walks::service::axis;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Operation;

/// The step `input` reads, when it reads one: the rows of a relation are made by no step.
fn made_of(input: Input) -> Option<StepId> {
    match input {
        Input::Step(id) => Some(id),
        Input::Relation(_) => None,
    }
}

/// The dependency order of an operation handled by the tree fragment executor.
fn dependencies(operation: &Operation) -> Result<(Option<StepId>, Option<StepId>), OperatingError> {
    Ok(match operation {
        Operation::Select { from, .. } | Operation::Project { from, .. } => (made_of(*from), None),
        Operation::Join { left, right, .. }
        | Operation::Antijoin { left, right, .. }
        | Operation::Union { left, right } => (made_of(*left), made_of(*right)),
        Operation::Axis { from, domain, .. } => (made_of(*from), made_of(*domain)),
        Operation::Extend { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::GroupCount { .. }
        | Operation::Replace { .. }
        | Operation::SpanBounds { .. }
        | Operation::SourceMatches { .. }
        | Operation::SpanBetween { .. }
        | Operation::OrderedMatch { .. }
        | Operation::OrderedDfs { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Words { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => return Err(OperatingError::WrongOperation),
    })
}

/// Makes the rows of the step `id` and of the lowered steps it reads, once.
///
/// # Errors
///
/// What a comparison of a step refuses.
pub(crate) fn make(
    run: &mut FragmentRun<'_, '_>,
    id: StepId,
    context: &mut OperatorContext<'_>,
) -> Result<(), OperatingError> {
    if run
        .room
        .ready
        .get(usize::try_from(id.0).unwrap_or(usize::MAX))
        .copied()
        == Some(Readiness::Made)
    {
        return Ok(());
    }
    let plan = run.plan;
    let lowering = run.lowering;
    let width = shape_at(lowering, id)?.width;
    let place = usize::try_from(id.0).unwrap_or(usize::MAX);
    // A step the pass over the file's elements decided for the rules of the call is its elements, and what it reads is not made.
    if selected(run.room, run.classes, id).is_some() {
        let mut slots = run
            .room
            .made
            .get_mut(place)
            .map(|made| std::mem::take(&mut made.slots))
            .ok_or(OperatingError::NotRun)?;
        slots.0.clear();
        if let Some(chosen) = selected(run.room, run.classes, id) {
            slots.0.extend_from_slice(&chosen.0);
        }
        if let Some(done) = run.room.made.get_mut(place) {
            *done = Positions { width, slots };
        }
        if let Some(ready) = run.room.ready.get_mut(place) {
            *ready = Readiness::Made;
        }
        return Ok(());
    }
    let operation = &plan
        .steps
        .get(place)
        .ok_or(OperatingError::NotRun)?
        .operation;
    // What a step reads is made first, the left side before the right: nothing on the left joins with nothing, so the right is
    // not made.
    let (first, second) = dependencies(operation)?;
    if let Some(first) = first {
        make(run, first, context)?;
    }
    let left_empty = first.is_some_and(|each| {
        run.room
            .made
            .get(usize::try_from(each.0).unwrap_or(usize::MAX))
            .is_some_and(|made| count(made).0 == 0)
    });
    let skips = left_empty
        && matches!(
            operation,
            Operation::Join { .. }
                | Operation::Antijoin { .. }
                | Operation::Axis { .. }
                | Operation::Select { .. }
                | Operation::Project { .. }
        );
    if !skips && let Some(second) = second {
        make(run, second, context)?;
    }
    let Some(made) = run.room.made.get_mut(place) else {
        return Err(OperatingError::NotRun);
    };
    let mut out = std::mem::take(&mut made.slots);
    out.0.clear();
    if !skips {
        let outcome = make_rows(run, id, operation, context, &mut out);
        if let Err(error) = outcome {
            if let Some(failed) = run.room.made.get_mut(place) {
                failed.slots = out;
            }
            return Err(error);
        }
    }
    if let Some(done) = run.room.made.get_mut(place) {
        *done = Positions { width, slots: out };
    }
    if let Some(ready) = run.room.ready.get_mut(place) {
        *ready = Readiness::Made;
    }
    Ok(())
}

/// Makes the rows of the step `id`, whose inputs are made, into `out`.
///
/// # Errors
///
/// What a comparison of the step refuses, or `NotRun` when the step is not one the tree can run.
fn make_rows(
    run: &mut FragmentRun<'_, '_>,
    id: StepId,
    operation: &Operation,
    context: &mut OperatorContext<'_>,
    out: &mut Slots,
) -> Result<(), OperatingError> {
    let tree = run.tree;
    let lowering = run.lowering;
    let work = Work {
        room: &mut *run.room,
        context,
    };
    match operation {
        Operation::Select {
            from: Input::Relation(_),
            comparisons,
            ..
        } => select_tree(&tree, comparisons, work.context, out),
        Operation::Select {
            from, comparisons, ..
        } => select_rows(&tree, lowering, step_of(*from)?, comparisons, work, out),
        Operation::Project {
            from,
            columns,
            distinct,
        } => {
            project(
                &tree,
                lowering,
                step_of(*from)?,
                ProjectedColumns {
                    columns,
                    repeats: *distinct,
                },
                work.room,
                out,
            );
            Ok(())
        }
        Operation::Join { .. } => join(&tree, lowering, operation, work, out),
        Operation::Axis { .. } => axis(&tree, lowering, operation, work, out),
        Operation::Antijoin {
            left,
            right,
            left_on,
            right_on,
        } => antijoin(
            &tree,
            lowering,
            Sides {
                left: step_of(*left)?,
                right: step_of(*right)?,
            },
            KeyColumns {
                left: left_on,
                right: right_on,
            },
            work.room,
            out,
        ),
        Operation::Union { left, right } => union(
            &tree,
            lowering,
            id,
            Sides {
                left: step_of(*left)?,
                right: step_of(*right)?,
            },
            work.room,
            out,
        ),
        Operation::Extend { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::GroupCount { .. }
        | Operation::Replace { .. }
        | Operation::SpanBounds { .. }
        | Operation::SourceMatches { .. }
        | Operation::SpanBetween { .. }
        | Operation::OrderedMatch { .. }
        | Operation::OrderedDfs { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Words { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => Err(OperatingError::WrongOperation),
    }
}

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
    make(run, input, context)?;
    let shape = shape_at(run.lowering, input)?;
    let tree = run.tree;
    let LoweredRoom {
        made,
        keys: flat,
        order,
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
    sorted(flat, Stride(stride.max(1)), order);
    let (flat, order) = (&flat.0, &order.0);
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
                let same =
                    last_distinct.is_some_and(|before| distinct_at(before) == distinct_at(each));
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
        let each = row(rows, crate::domain::table_cells::types::RowIndex(first));
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
