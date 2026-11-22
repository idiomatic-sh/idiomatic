//! The `selection_pushing` service's placing part: the filters a push makes and the one splice that places them before the step
//! they take the place of.

use super::super::super::types::PushedFilter;
use super::super::super::types::SplicePlaces;
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
    first: StepId,
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
    let at = place_after(first, &before);
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

/// The place the steps a deferred splice makes are named from while a push makes them (`insert_deferred`): far past any
/// step of a plan, so a place from it is a step of the splice and no other.
pub(crate) const FRESH: StepId = StepId(u32::MAX / 2);

/// The order of a plan whose pushes were deferred (prototype ryo-fmx3.1): a step of the plan as the pass began stands at
/// its place; the steps a splice made stand at the end of the plan, and `keys` holds where each goes, a path: the place of
/// the step they were made before, then for each splice on the way the number of the splice made before that step and the
/// step's place in it. `materialized` sorts the plan by it into the plan the insertions, made one by one, give.
#[derive(Clone, Debug, Default)]
pub(crate) struct PlanOrder {
    /// The number of steps the plan had when the pass began.
    pub original: usize,
    /// The path of each step made since, by its place less `original`; the last part of every path is `u32::MAX`.
    pub keys: Vec<Vec<u32>>,
    /// For each step, the number of splices made before it so far.
    pub attached: Vec<u32>,
}

/// Compares the places two steps take in the plan once it is materialized.
pub(crate) fn order_of(order: &PlanOrder, left: usize, right: usize) -> std::cmp::Ordering {
    let own = |place: usize| [u32::try_from(place).unwrap_or(u32::MAX), u32::MAX];
    let (left_own, right_own) = (own(left), own(right));
    let left_key: &[u32] = if left < order.original {
        &left_own
    } else {
        order.keys.get(left - order.original).map_or(&left_own, Vec::as_slice)
    };
    let right_key: &[u32] = if right < order.original {
        &right_own
    } else {
        order.keys.get(right - order.original).map_or(&right_own, Vec::as_slice)
    };
    left_key.cmp(right_key)
}

/// The splice of `insert_ordered`, deferred: `before` is put at the end of the plan and the step at `place` becomes
/// `filter`, so no step is renumbered. `before` and `filter` name the steps of the splice from `FRESH` (the first of
/// `before` at `FRESH`, `filter` at `FRESH` + its length), and are given their places at the end; where they go is kept
/// in `order`. Answers how a place the push named is named now.
pub(crate) fn insert_deferred(
    steps: &mut Vec<PlanStep>,
    order: &mut PlanOrder,
    place: StepIndex,
    mut before: Vec<PlanStep>,
    mut filter: PlanStep,
) -> SplicePlaces {
    let base = steps.len();
    let count = before.len();
    let places = SplicePlaces {
        at: place.0,
        count,
        base,
    };
    for step in before.iter_mut().chain(std::iter::once(&mut filter)) {
        each_input(&mut step.operation, &mut |input| {
            if let Input::Step(id) = input {
                *id = placed_as(&places, *id);
            }
        });
    }
    if order.attached.len() < base {
        order.attached.resize(base, 0);
    }
    let made = order.attached.get(place.0).copied().unwrap_or(0);
    if let Some(slot) = order.attached.get_mut(place.0) {
        *slot += 1;
    }
    let mut path: Vec<u32> = if place.0 < order.original {
        vec![u32::try_from(place.0).unwrap_or(u32::MAX)]
    } else {
        order
            .keys
            .get(place.0 - order.original)
            .map(|key| key.get(..key.len().saturating_sub(1)).unwrap_or_default().to_vec())
            .unwrap_or_default()
    };
    path.push(made);
    for number in 0..count {
        let mut key = path.clone();
        key.push(u32::try_from(number).unwrap_or(u32::MAX));
        key.push(u32::MAX);
        order.keys.push(key);
        order.attached.push(0);
    }
    if let Some(step) = steps.get_mut(place.0) {
        *step = filter;
    }
    steps.extend(before);
    places
}

/// The place a step a deferred push named stands at: a place from `FRESH` is a step of the splice, at the end of the
/// plan, or the step it was made before; any other place is the step there.
pub(crate) fn placed_as(places: &SplicePlaces, id: StepId) -> StepId {
    if id.0 < FRESH.0 {
        return id;
    }
    let number = usize::try_from(id.0 - FRESH.0).unwrap_or(usize::MAX);
    let at = if number < places.count {
        places.base + number
    } else {
        places.at
    };
    StepId(u32::try_from(at).unwrap_or(u32::MAX))
}

/// The plan of a pass whose pushes were deferred in the order the insertions give: every step sorted by its path, and every
/// input and root renumbered.
pub(crate) fn materialized(
    steps: Vec<PlanStep>,
    roots: &mut [StepId],
    order: &PlanOrder,
) -> Vec<PlanStep> {
    let mut sorted: Vec<usize> = (0..steps.len()).collect();
    sorted.sort_by(|left, right| order_of(order, *left, *right));
    let mut now = vec![0_u32; steps.len()];
    for (at, place) in sorted.iter().enumerate() {
        if let Some(slot) = now.get_mut(*place) {
            *slot = u32::try_from(at).unwrap_or(u32::MAX);
        }
    }
    let mut taken: Vec<Option<PlanStep>> = steps.into_iter().map(Some).collect();
    let mut made = Vec::with_capacity(taken.len());
    for place in sorted {
        if let Some(mut step) = taken.get_mut(place).and_then(Option::take) {
            each_input(&mut step.operation, &mut |input| {
                if let Input::Step(id) = input
                    && let Some(at) = now.get(usize::try_from(id.0).unwrap_or(usize::MAX))
                {
                    id.0 = *at;
                }
            });
            made.push(step);
        }
    }
    for root in roots.iter_mut() {
        if let Some(at) = now.get(usize::try_from(root.0).unwrap_or(usize::MAX)) {
            root.0 = *at;
        }
    }
    made
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
