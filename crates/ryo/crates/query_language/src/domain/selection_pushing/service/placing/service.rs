//! The `selection_pushing` service's placing part: the filters a push makes and the one splice that places them before the step
//! they take the place of.

use super::super::super::types::PushedFilter;
use crate::domain::planning::service::planned_scope;
use crate::domain::rewriting::types::StepIndex;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::service::each_input;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// A filter of `from` by `comparisons`, made where `from` is.
pub(crate) fn filter_of(
    steps: &[PlanStep],
    from: Input,
    comparisons: Vec<PlanComparison>,
) -> Option<PlanStep> {
    Some(PlanStep {
        operation: Operation::Select {
            from,
            comparisons,
            kept: Vec::new(),
        },
        scope: planned_scope(steps, from)?,
    })
}

/// What takes the place of `filtering`, the filter at `reader`, once `before` and the union or the join of them, `combined`, are
/// made: `combined` itself when nothing is left for the filter to do, else a filter of it by `rest` keeping `kept`.
pub(crate) fn finished(
    reader: StepIndex,
    mut before: Vec<PlanStep>,
    combined: PlanStep,
    filtering: &PlanStep,
    rest: Vec<PlanComparison>,
    kept: Vec<ColumnIndex>,
) -> PushedFilter {
    if rest.is_empty() && kept.is_empty() {
        return PushedFilter {
            reader,
            before,
            filter: PlanStep {
                operation: combined.operation,
                scope: filtering.scope,
            },
        };
    }
    let at = place_after(StepId(u32::try_from(reader.0).unwrap_or(u32::MAX)), &before);
    before.push(combined);
    PushedFilter {
        reader,
        before,
        filter: PlanStep {
            operation: Operation::Select {
                from: at,
                comparisons: rest,
                kept,
            },
            scope: filtering.scope,
        },
    }
}

/// The place the next step made before the filter at `first` takes, after `made`.
pub(crate) fn place_after(first: StepId, made: &[PlanStep]) -> Input {
    Input::Step(StepId(
        first.0 + u32::try_from(made.len()).unwrap_or(u32::MAX),
    ))
}

/// The plan with `before` made just before the step at `place`, which becomes `filter`, and every later step and root renumbered.
/// The one splice of the rewrites that place a step's work before it: placement here, and the lookups `push_lookups` splits.
pub(crate) fn insert_before(
    steps: &mut Vec<PlanStep>,
    roots: &mut [StepId],
    place: StepIndex,
    before: Vec<PlanStep>,
    filter: PlanStep,
) {
    let count = u32::try_from(before.len()).unwrap_or(u32::MAX);
    let first = u32::try_from(place.0).unwrap_or(u32::MAX);
    let shift = |input: &mut Input| {
        if let Input::Step(id) = input
            && id.0 >= first
        {
            id.0 += count;
        }
    };
    for step in steps.iter_mut() {
        each_input(&mut step.operation, &mut |input| shift(input));
    }
    for root in roots.iter_mut() {
        if root.0 >= first {
            root.0 += count;
        }
    }
    // The steps made here already name their places after the shift: what came before the filter, and each other.
    if let Some(step) = steps.get_mut(place.0) {
        *step = filter;
    }
    for (offset, step) in before.into_iter().enumerate() {
        steps.insert(place.0 + offset, step);
    }
}

/// `insert_before` for a plan in which every step reads only steps before it: no step up to `place` names a step from
/// `place` on, so only the steps after it are renumbered, and the steps made are spliced in at once. The same plan as
/// `insert_before` gives for such a plan.
pub(crate) fn insert_ordered(
    steps: &mut Vec<PlanStep>,
    roots: &mut [StepId],
    place: StepIndex,
    before: Vec<PlanStep>,
    filter: PlanStep,
) {
    let count = u32::try_from(before.len()).unwrap_or(u32::MAX);
    let first = u32::try_from(place.0).unwrap_or(u32::MAX);
    for step in steps.iter_mut().skip(place.0.saturating_add(1)) {
        each_input(&mut step.operation, &mut |input| {
            if let Input::Step(id) = input
                && id.0 >= first
            {
                id.0 += count;
            }
        });
    }
    for root in roots.iter_mut() {
        if root.0 >= first {
            root.0 += count;
        }
    }
    if let Some(step) = steps.get_mut(place.0) {
        *step = filter;
    }
    let _none = steps.splice(place.0..place.0, before);
}

/// The key column of the other input that the key column `column` of one input meets, `own` and `other` being their keys in
/// order: every row the join or the antijoin matches holds the same values in the two.
pub(crate) fn met_key(
    own: &[ColumnIndex],
    other: &[ColumnIndex],
    column: ColumnIndex,
) -> Option<ColumnIndex> {
    let at = own.iter().position(|key| *key == column)?;
    other.get(at).copied()
}
