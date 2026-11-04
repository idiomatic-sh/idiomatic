//! The `fragment` service: the lowered steps of a plan run over the positions of the elements of a file, each step made once, a value
//! made only where a comparison, a key or the step that takes the rows asks for one.

use super::types::FragmentRun;
use crate::domain::operator::contract::types::PrimitiveSemantics;
mod join;
pub(crate) mod set_rows;
use crate::domain::lowering::service::{shape_at, step_of};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::positions::service::{count, row, selected};
use crate::domain::positions::types::{
    KeyColumns, Positions, Readiness, Sides, Slots, Stride, Work,
};
use crate::domain::selections::service::{project, select_rows, select_tree};
use crate::domain::selections::types::ProjectedColumns;
use crate::domain::walks::service::axis;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Operation;

/// The step `input` reads, when it reads one: the rows of a relation are made by no step.
fn made_of(input: Input) -> Option<StepId> {
    match input {
        Input::Step(id) => Some(id),
        Input::Relation(_) | Input::BoundObservation(_) => None,
    }
}

/// The dependency order of an operation handled by the tree fragment executor.
struct StepDependencies {
    first: Option<StepId>,
    second: Option<StepId>,
}
fn dependencies(operation: &Operation) -> Result<StepDependencies, OperatingError> {
    let (first, second) = match operation {
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
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
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
    };
    Ok(StepDependencies { first, second })
}

/// Makes the rows of the step `id` and of the lowered steps it reads, once.
///
/// # Errors
///
/// What a comparison of a step refuses.
pub(crate) fn make(
    semantics: &dyn PrimitiveSemantics,
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
    let StepDependencies { first, second } = dependencies(operation)?;
    if let Some(first) = first {
        make(semantics, run, first, context)?;
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
        make(semantics, run, second, context)?;
    }
    let Some(made) = run.room.made.get_mut(place) else {
        return Err(OperatingError::NotRun);
    };
    let mut out = std::mem::take(&mut made.slots);
    out.0.clear();
    if !skips {
        let outcome = make_rows(semantics, run, id, operation, context, &mut out);
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
    semantics: &dyn PrimitiveSemantics,
    run: &mut FragmentRun<'_, '_>,
    id: StepId,
    operation: &Operation,
    context: &mut OperatorContext<'_>,
    out: &mut Slots,
) -> Result<(), OperatingError> {
    let tree = run.tree;
    let lowering = run.lowering;
    let work = Work {
        derived: &mut *run.derived,
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
        } => project(
            work.derived,
            &tree,
            lowering,
            step_of(*from)?,
            ProjectedColumns {
                columns,
                repeats: *distinct,
            },
            work.room,
            out,
        ),
        Operation::Join { .. } => join::run(semantics, &tree, lowering, operation, work, out),
        Operation::Axis { .. } => axis(&tree, lowering, operation, work, out),
        Operation::Antijoin {
            left,
            right,
            left_on,
            right_on,
        } => set_rows::antijoin(
            semantics,
            work.derived,
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
        Operation::Union { left, right } => set_rows::union(
            semantics,
            work.derived,
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
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
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
