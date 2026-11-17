//! The `selection_pushing` service: free functions named by verb over the filters of a union or a join made on its inputs.

use super::types::PushedFilter;
use super::types::SplitComparisons;
use crate::domain::planning::service::planned_scope;
use crate::domain::pruning::service::without_unused_roots;
use crate::domain::rewriting::types::Planned;
use crate::domain::rewriting::types::StepIndex;
use crate::domain::widths::service::width_of;
use ryo_values::domain::plan_parts::service::column_in;
use ryo_values::domain::plan_parts::service::fixed_prefix;
use ryo_values::domain::plan_parts::service::remapped_comparison;
use ryo_values::domain::plan_parts::service::rides_the_walk;
use ryo_values::domain::plan_parts::service::step_in;
use ryo_values::domain::plan_parts::service::view_source;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::RowWidth;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::service::{each_input, inputs_of, named};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// The plan with each filter of a join placed on the join's inputs, and the filters of a union made on both its inputs: the join or
/// the union combines only the rows the filter lets through. A filter of a join moves the comparisons of its infallible prefix
/// (`fixed_prefix`) that test one input: one of the left is a filter of the left; one of the right rides the probe when the probe
/// walks the tree by a node (`rides_the_walk`), and is a filter of the right otherwise. Each filter of a join is placed on its own
/// copy of the join, and a reader of the join that is no filter keeps reading the join. A union's filters move, their infallible
/// prefix alone, only when every reader of the union is such a filter. The rest of a filter stays after the copy, in its order, so
/// a comparison that may fail runs on the rows it ran on: `σ(L ⋈ R) = σL ⋈ σR` and `σ(A ∪ B) = σA ∪ σB`. Done again on the
/// copies, until no filter moves; each move takes a filter strictly down, so it ends.
#[must_use]
pub(crate) fn push_selections(steps: Vec<PlanStep>, mut emitted: StepId) -> Planned {
    let steps = push_selections_roots(steps, std::slice::from_mut(&mut emitted));
    Planned { steps, emitted }
}

/// The same rewrite with every externally observable root protected and renumbered together.
pub(crate) fn push_selections_roots(
    mut steps: Vec<PlanStep>,
    roots: &mut [StepId],
) -> Vec<PlanStep> {
    loop {
        let mut moved = false;
        let mut readers = readers_of(&steps);
        for place in (0..steps.len()).rev() {
            let id = StepId(u32::try_from(place).unwrap_or(u32::MAX));
            if roots.contains(&id) {
                continue;
            }
            let pushed = readers
                .get(place)
                .map(|reading| placed(&steps, id, reading))
                .unwrap_or_default();
            if pushed.is_empty() {
                continue;
            }
            // The last reader first, so the places of the ones before it stay as they are.
            for PushedFilter {
                reader,
                before,
                filter,
            } in pushed.into_iter().rev()
            {
                insert_before(&mut steps, roots, reader, before, filter);
            }
            readers = readers_of(&steps);
            moved = true;
        }
        if !moved {
            return steps;
        }
        steps = without_unused_roots(steps, roots);
    }
}

/// For each step, the places of the steps that read it.
fn readers_of(steps: &[PlanStep]) -> Vec<Vec<StepIndex>> {
    let mut readers = vec![Vec::new(); steps.len()];
    for (place, step) in steps.iter().enumerate() {
        for input in named(&inputs_of(&step.operation)) {
            if let Input::Step(id) = *input
                && let Some(reading) = readers.get_mut(usize::try_from(id.0).unwrap_or(usize::MAX))
                && reading.last() != Some(&StepIndex(place))
            {
                reading.push(StepIndex(place));
            }
        }
    }
    readers
}

/// What takes the place of each of `readers`, the steps that read the union or the join `id`, whose comparisons move: for a join,
/// each filter that places some on an input; for a union, every reader, when each is a filter whose infallible prefix moves, and
/// none otherwise.
fn placed(steps: &[PlanStep], id: StepId, readers: &[StepIndex]) -> Vec<PushedFilter> {
    let Some(made) = step_in(steps, id) else {
        return Vec::new();
    };
    // The readers that are filters of the step, made where it is.
    let filtering: Vec<StepIndex> = readers
        .iter()
        .copied()
        .filter(|reader| {
            steps.get(reader.0).is_some_and(|step| {
                step.scope == made.scope
                    && matches!(step.operation, Operation::Select { from, .. } if from == Input::Step(id))
            })
        })
        .collect();
    if let Operation::Union { left, right } = &made.operation {
        if filtering.is_empty() || filtering.len() != readers.len() {
            return Vec::new();
        }
        filtering
            .iter()
            .map(|reader| pushed_union(steps, id, *reader, *left, *right))
            .collect::<Option<Vec<_>>>()
            .unwrap_or_default()
    } else if let Operation::Join { reach: None, .. } = &made.operation {
        filtering
            .iter()
            .filter_map(|reader| pushed_join(steps, id, *reader))
            .collect()
    } else {
        Vec::new()
    }
}

/// The comparisons of a filter of the join that answers `output` through `view`, by where they are decided.
fn split_comparisons(
    comparisons: &[PlanComparison],
    output: &[JoinColumn],
    view: &[ColumnIndex],
) -> SplitComparisons {
    let side_column = |side: Side| {
        move |column: ColumnIndex| {
            let made = column_in(output, column)?;
            if made.side != side {
                return None;
            }
            match side {
                Side::Left => Some(made.column),
                Side::Right => view_source(view, made.column),
            }
        }
    };
    let prefix = fixed_prefix(comparisons, RowWidth(output.len())).0;
    let mut split = SplitComparisons::default();
    for comparison in prefix {
        if let Some(left) = remapped_comparison(comparison, &side_column(Side::Left)) {
            split.left.push(left);
        } else if let Some(right) = remapped_comparison(comparison, &side_column(Side::Right)) {
            split.right.push(right);
        } else {
            split.rest.push(comparison.clone());
        }
    }
    split.rest.extend(
        comparisons
            .get(prefix.len()..)
            .unwrap_or_default()
            .iter()
            .cloned(),
    );
    split
}

/// A filter of `from` by `comparisons`, made where `from` is.
fn filter_of(
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

/// What takes the place of the filter at `reader` of the union `id` of `left` and `right`, when its infallible prefix moves.
fn pushed_union(
    steps: &[PlanStep],
    id: StepId,
    reader: StepIndex,
    left: Input,
    right: Input,
) -> Option<PushedFilter> {
    let made = step_in(steps, id)?;
    let filtering = steps.get(reader.0)?;
    let Operation::Select {
        comparisons, kept, ..
    } = &filtering.operation
    else {
        return None;
    };
    let width = usize::try_from(width_of(steps, Input::Step(id))?.0).unwrap_or(usize::MAX);
    let prefix = fixed_prefix(comparisons, RowWidth(width)).0;
    if prefix.is_empty() {
        return None;
    }
    let first = StepId(u32::try_from(reader.0).unwrap_or(u32::MAX));
    let mut before = Vec::new();
    let filtered_left = place_after(first, &before);
    before.push(filter_of(steps, left, prefix.to_vec())?);
    let filtered_right = place_after(first, &before);
    before.push(filter_of(steps, right, prefix.to_vec())?);
    let combined = PlanStep {
        operation: Operation::Union {
            left: filtered_left,
            right: filtered_right,
        },
        scope: made.scope,
    };
    let rest = comparisons.get(prefix.len()..).unwrap_or_default().to_vec();
    Some(finished(
        reader,
        before,
        combined,
        filtering,
        rest,
        kept.clone(),
    ))
}

/// What takes the place of the filter at `reader` of the join `id`, when some of its comparisons are placed on an input.
fn pushed_join(steps: &[PlanStep], id: StepId, reader: StepIndex) -> Option<PushedFilter> {
    let made = step_in(steps, id)?;
    let filtering = steps.get(reader.0)?;
    let Operation::Select {
        comparisons, kept, ..
    } = &filtering.operation
    else {
        return None;
    };
    let Operation::Join {
        left,
        right,
        left_on,
        right_on,
        output,
        filter,
        view,
        reach: None,
    } = &made.operation
    else {
        return None;
    };
    let split = split_comparisons(comparisons, output, view);
    if split.left.is_empty() && split.right.is_empty() {
        return None;
    }
    let first = StepId(u32::try_from(reader.0).unwrap_or(u32::MAX));
    let mut before = Vec::new();
    let mut from = |input: Input, moved: Vec<PlanComparison>| -> Option<Input> {
        if moved.is_empty() {
            return Some(input);
        }
        let at = place_after(first, &before);
        before.push(filter_of(steps, input, moved)?);
        Some(at)
    };
    let placed_left = from(*left, split.left)?;
    // The right comparisons ride the probe where it walks the tree by a node; elsewhere they are a filter of the right, so the
    // probe's index is the filter's node and is shared as one.
    let walked: Vec<PlanComparison> = filter.iter().chain(&split.right).cloned().collect();
    let rides = !split.right.is_empty()
        && matches!(right, Input::Relation(relation)
            if rides_the_walk(*relation, right_on, view, &walked).is_some());
    let (placed_right, placed_filter) = if rides {
        (*right, walked)
    } else {
        (from(*right, split.right)?, filter.clone())
    };
    // A filter that only keeps columns, once its comparisons are placed, is the join's answer itself.
    let (answer, kept) = if split.rest.is_empty() && !kept.is_empty() {
        (
            kept.iter()
                .map(|column| column_in(output, *column).copied())
                .collect::<Option<Vec<_>>>()?,
            Vec::new(),
        )
    } else {
        (output.clone(), kept.clone())
    };
    let combined = PlanStep {
        operation: Operation::Join {
            left: placed_left,
            right: placed_right,
            left_on: left_on.clone(),
            right_on: right_on.clone(),
            output: answer,
            filter: placed_filter,
            view: view.clone(),
            reach: None,
        },
        scope: made.scope,
    };
    Some(finished(
        reader, before, combined, filtering, split.rest, kept,
    ))
}

/// What takes the place of `filtering`, the filter at `reader`, once `before` and the union or the join of them, `combined`, are
/// made: `combined` itself when nothing is left for the filter to do, else a filter of it by `rest` keeping `kept`.
fn finished(
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
fn place_after(first: StepId, made: &[PlanStep]) -> Input {
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
