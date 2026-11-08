//! The `selection_pushing` service: free functions named by verb over the filters of a union or a join made on its inputs.

use super::types::PushedFilter;
use super::types::SplitComparisons;
use crate::domain::fusing::service::source_column;
use crate::domain::lineage::service::column_kind;
use crate::domain::planning::service::inputs_of;
use crate::domain::planning::service::planned_scope;
use crate::domain::pruning::service::without_unused_roots;
use crate::domain::rewriting::types::Planned;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::service::column_in;
use ryo_values::domain::plan_parts::service::columns_of;
use ryo_values::domain::plan_parts::service::remapped;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::service::each_input;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// The plan with the filters of a union made on both its inputs, `σ(A ∪ B) = σA ∪ σB`, and the comparisons of a filter of a join that
/// test the columns of one input made on that input, `σ(L ⋈ R) = L ⋈ σR`: the union or the join combines only the rows the filter
/// lets through. Done for a union or a join every reader of which is such a filter, one copy for each, so the whole one is no longer
/// made; and again on the copies, until no filter moves.
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
            if roots.contains(&id) || !filters_only(&steps, place, readers.get(place)) {
                continue;
            }
            let Some(reading) = readers.get(place) else {
                continue;
            };
            let Some(pushed) = reading
                .iter()
                .map(|reader| pushed_filter(&steps, id, *reader).map(|each| (*reader, each)))
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            // The last reader first, so the places of the ones before it stay as they are.
            for (reader, PushedFilter { before, filter }) in pushed.into_iter().rev() {
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
fn readers_of(steps: &[PlanStep]) -> Vec<Vec<usize>> {
    let mut readers = vec![Vec::new(); steps.len()];
    for (place, step) in steps.iter().enumerate() {
        for input in inputs_of(&step.operation) {
            if let Input::Step(id) = input
                && let Some(reading) = readers.get_mut(usize::try_from(id.0).unwrap_or(usize::MAX))
                && reading.last() != Some(&place)
            {
                reading.push(place);
            }
        }
    }
    readers
}

/// Whether the step at `place` is a union or a join, and `readers`, the steps that read it, are some, each a filter of it made where it
/// is.
fn filters_only(steps: &[PlanStep], place: usize, readers: Option<&Vec<usize>>) -> bool {
    let Some(made) = steps.get(place) else {
        return false;
    };
    let Some(readers) = readers.filter(|readers| !readers.is_empty()) else {
        return false;
    };
    // One copy for each of several filters pays where the whole would be large for the rows they keep: a union, or a join keyed on the
    // file alone, which pairs every row of a file with every row of it on the other side.
    let copied = match &made.operation {
        Operation::Union { .. } => true,
        Operation::Join {
            left,
            left_on,
            reach: None,
            ..
        } => {
            readers.len() == 1
                || left_on
                    .iter()
                    .all(|key| column_kind(steps, *left, *key) == Some(CellKind::File))
        }
        Operation::Select { .. }
        | Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Words { .. }
        | Operation::Axis { .. }
        | Operation::Join { .. }
        | Operation::Closure { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. }
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. } => false,
    };
    copied && readers.iter().all(|reader| {
        steps.get(*reader).is_some_and(|step| {
            step.scope == made.scope
                && matches!(step.operation, Operation::Select { from: Input::Step(id), .. } if usize::try_from(id.0).ok() == Some(place))
        })
    })
}

/// Whether a comparison tests a column.
fn tests_a_column(comparison: &PlanComparison) -> bool {
    !columns_of(&comparison.left).is_empty() || !columns_of(&comparison.right).is_empty()
}

/// The comparison with each column it tests named by `map`, when `map` names every one.
fn renamed(
    comparison: &PlanComparison,
    map: &dyn Fn(ColumnIndex) -> Option<ColumnIndex>,
) -> Option<PlanComparison> {
    Some(PlanComparison {
        test: comparison.test,
        left: remapped(&comparison.left, &map)?,
        right: remapped(&comparison.right, &map)?,
        origin: comparison.origin,
    })
}

/// The comparisons of a filter of the join that answers `output` through `view`, by the input whose columns they test.
fn split_comparisons(
    comparisons: &[PlanComparison],
    output: &[ryo_values::domain::plan_parts::types::JoinColumn],
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
                Side::Right => source_column(view, made.column),
            }
        }
    };
    let mut split = SplitComparisons::default();
    for comparison in comparisons {
        if !tests_a_column(comparison) {
            split.rest.push(comparison.clone());
        } else if let Some(left) = renamed(comparison, &side_column(Side::Left)) {
            split.left.push(left);
        } else if let Some(right) = renamed(comparison, &side_column(Side::Right)) {
            split.right.push(right);
        } else {
            split.rest.push(comparison.clone());
        }
    }
    split
}

/// A filter of `from` by `comparisons`, made where `from` is.
fn filter_of(steps: &[PlanStep], from: Input, comparisons: Vec<PlanComparison>) -> Option<PlanStep> {
    Some(PlanStep {
        operation: Operation::Select {
            from,
            comparisons,
            kept: Vec::new(),
        },
        scope: planned_scope(steps, from)?,
    })
}

/// What takes the place of the filter at `reader` of the union or the join `id`, when some of its comparisons move.
fn pushed_filter(steps: &[PlanStep], id: StepId, reader: usize) -> Option<PushedFilter> {
    let made = steps.get(usize::try_from(id.0).ok()?)?;
    let filtering = steps.get(reader)?;
    let Operation::Select {
        comparisons, kept, ..
    } = &filtering.operation
    else {
        return None;
    };
    let first = StepId(u32::try_from(reader).ok()?);
    let next = |offset: u32| Input::Step(StepId(first.0 + offset));
    let mut before = Vec::new();
    let (combined, rest) = match &made.operation {
        Operation::Union { left, right } => {
            if !comparisons.iter().any(tests_a_column) {
                return None;
            }
            before.push(filter_of(steps, *left, comparisons.clone())?);
            before.push(filter_of(steps, *right, comparisons.clone())?);
            (
                Operation::Union {
                    left: next(0),
                    right: next(1),
                },
                Vec::new(),
            )
        }
        Operation::Join {
            left,
            right,
            left_on,
            right_on,
            output,
            filter,
            view,
            reach: None,
        } => {
            let split = split_comparisons(comparisons, output, view);
            if split.left.is_empty() && split.right.is_empty() {
                return None;
            }
            let mut from = |input: Input, moved: Vec<PlanComparison>| -> Option<Input> {
                if moved.is_empty() {
                    return Some(input);
                }
                let at = next(u32::try_from(before.len()).ok()?);
                before.push(filter_of(steps, input, moved)?);
                Some(at)
            };
            let left = from(*left, split.left)?;
            let right = from(*right, split.right)?;
            (
                Operation::Join {
                    left,
                    right,
                    left_on: left_on.clone(),
                    right_on: right_on.clone(),
                    output: output.clone(),
                    filter: filter.clone(),
                    view: view.clone(),
                    reach: None,
                },
                split.rest,
            )
        }
        Operation::Select { .. }
        | Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Words { .. }
        | Operation::Axis { .. }
        | Operation::Join { .. }
        | Operation::Closure { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. }
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. } => return None,
    };
    // The union or the join is the filter's own step when nothing is left for the filter to do.
    let filter = if rest.is_empty() && kept.is_empty() {
        PlanStep {
            operation: combined,
            scope: filtering.scope,
        }
    } else {
        let at = next(u32::try_from(before.len()).ok()?);
        before.push(PlanStep {
            operation: combined,
            scope: made.scope,
        });
        PlanStep {
            operation: Operation::Select {
                from: at,
                comparisons: rest,
                kept: kept.clone(),
            },
            scope: filtering.scope,
        }
    };
    Some(PushedFilter { before, filter })
}

/// The plan with `before` made just before the step at `place`, which becomes `filter`, and every later step and root renumbered.
fn insert_before(
    steps: &mut Vec<PlanStep>,
    roots: &mut [StepId],
    place: usize,
    before: Vec<PlanStep>,
    filter: PlanStep,
) {
    let count = u32::try_from(before.len()).unwrap_or(u32::MAX);
    let first = u32::try_from(place).unwrap_or(u32::MAX);
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
    if let Some(step) = steps.get_mut(place) {
        *step = filter;
    }
    for (offset, step) in before.into_iter().enumerate() {
        steps.insert(place + offset, step);
    }
}
