//! The `selection_pushing` service's factoring part: the comparisons every filter of an antijoin, a projection, an extension or
//! a filter shares, factored out of them and placed below it once.

use super::super::super::types::Below;
use super::super::super::types::Residual;
use super::super::super::types::SharedPush;
use super::super::placing::service::{filter_of, met_key, place_after, placed_as};
use crate::domain::fusing::service::fuse;
use crate::domain::rewriting::types::StepIndex;
use crate::domain::selection_pushing::types::SplicePlaces;
use crate::domain::widths::service::width_of;
use ryo_values::domain::plan_parts::service::column_in;
use ryo_values::domain::plan_parts::service::fixed_prefix;
use ryo_values::domain::plan_parts::service::remapped_comparison;
use ryo_values::domain::plan_parts::service::step_in;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::RowWidth;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;
use ryo_values::domain::written_sites::types::StepWritten;

/// The comparisons every filter of the step `id` shares, placed below it once, when each reader of the step is a filter of it
/// and the step is an antijoin, a projection, an extension or a filter: every filter keeps what is its own, after the step made
/// again over its filtered inputs. No copy of the step is made, so filters many steps read do not grow the plan.
pub(crate) fn shared_push(
    steps: &[PlanStep],
    id: StepId,
    readers: &[StepIndex],
    fresh: Option<StepId>,
) -> Option<SharedPush> {
    let made = step_in(steps, id)?;
    let below: fn(&[PlanStep], StepId, &[PlanComparison], StepId, &StepWritten) -> Option<Below> =
        if matches!(made.operation, Operation::Antijoin { .. }) {
            below_antijoin
        } else if matches!(made.operation, Operation::Project { .. }) {
            below_project
        } else if matches!(made.operation, Operation::Extend { .. }) {
            below_extend
        } else if matches!(made.operation, Operation::Select { .. }) {
            below_select
        } else {
            return None;
        };
    let width =
        RowWidth(usize::try_from(width_of(steps, Input::Step(id))?.0).unwrap_or(usize::MAX));
    let mut residuals = Vec::new();
    for reader in readers {
        let step = steps.get(reader.0)?;
        let Operation::Select {
            from,
            comparisons,
            kept,
        } = &step.operation
        else {
            return None;
        };
        if *from != Input::Step(id) || step.scope != made.scope {
            return None;
        }
        residuals.push(Residual {
            reader: *reader,
            comparisons: comparisons.clone(),
            kept: kept.clone(),
            scope: step.scope,
            written: step.written.clone(),
        });
    }
    let (head, others) = residuals.split_first()?;
    let common: Vec<PlanComparison> = fixed_prefix(&head.comparisons, width)
        .0
        .iter()
        .filter(|comparison| {
            others.iter().all(|other| {
                fixed_prefix(&other.comparisons, width)
                    .0
                    .contains(comparison)
            })
        })
        .cloned()
        .collect();
    if common.is_empty() {
        return None;
    }
    let below = below(
        steps,
        id,
        &common,
        fresh.unwrap_or(StepId(u32::try_from(head.reader.0).unwrap_or(u32::MAX))),
        &head.written,
    )?;
    if below.moved.is_empty() {
        return None;
    }
    for residual in &mut residuals {
        residual.comparisons = without(&residual.comparisons, &below.moved);
    }
    Some(SharedPush { below, residuals })
}

/// `comparisons` without one of each of `moved`, the rest in their order.
fn without(comparisons: &[PlanComparison], moved: &[PlanComparison]) -> Vec<PlanComparison> {
    let mut left: Vec<&PlanComparison> = moved.iter().collect();
    comparisons
        .iter()
        .filter(|comparison| {
            let found = left.iter().position(|each| *each == *comparison);
            if let Some(at) = found {
                let _moved = left.swap_remove(at);
            }
            found.is_none()
        })
        .cloned()
        .collect()
}

/// The plan with `push` made: the step made again below the filters, and what is made before it, just before the first filter,
/// which then reads it, and each other filter reading it too, by `splice`, which answers where the steps it made stand. The
/// push named its steps from `fresh` (the first filter's place when none): `splice` renumbers every later step and root, or
/// defers the push (`insert_deferred`).
pub(crate) fn shared_into(
    steps: &mut Vec<PlanStep>,
    push: SharedPush,
    fresh: Option<StepId>,
    splice: &mut dyn FnMut(&mut Vec<PlanStep>, StepIndex, Vec<PlanStep>, PlanStep) -> Option<SplicePlaces>,
) -> Option<()> {
    let SharedPush {
        below: Below {
            mut before,
            combined,
            ..
        },
        residuals,
    } = push;
    let (head, others) = residuals.split_first()?;
    let first = fresh.unwrap_or(StepId(u32::try_from(head.reader.0).unwrap_or(u32::MAX)));
    let at = place_after(first, &before);
    let residual = |residual: &Residual, from: Input| PlanStep {
        operation: Operation::Select {
            from,
            comparisons: residual.comparisons.clone(),
            kept: residual.kept.clone(),
        },
        scope: residual.scope,
        written: residual.written.clone(),
    };
    // A first filter with nothing of its own left is the step made again itself.
    let filter = if head.comparisons.is_empty() && head.kept.is_empty() {
        PlanStep {
            operation: combined.operation,
            scope: head.scope,
            written: combined.written,
        }
    } else {
        before.push(combined);
        residual(head, at)
    };
    let count = before.len();
    let deferred = splice(steps, head.reader, before, filter);
    let (shift, from) = match (&deferred, at) {
        (Some(places), Input::Step(id)) => (0, Input::Step(placed_as(places, id))),
        _ => (count, at),
    };
    for other in others {
        if let Some(step) = steps.get_mut(other.reader.0 + shift) {
            *step = residual(other, from);
        }
    }
    Some(())
}

/// The antijoin `id` over its inputs filtered by `comparisons`, of its left's columns: an antijoin keeps left rows as they are,
/// so each is a filter of the left, and also of the right when it tests only keys, which a right row that removes a left row
/// holds the same values in: `σ(L ▷ R) = σL ▷ σR`.
fn below_antijoin(
    steps: &[PlanStep],
    id: StepId,
    comparisons: &[PlanComparison],
    first: StepId,
    written: &StepWritten,
) -> Option<Below> {
    let made = step_in(steps, id)?;
    let Operation::Antijoin {
        left,
        right,
        left_on,
        right_on,
    } = &made.operation
    else {
        return None;
    };
    let right_key = |column: ColumnIndex| met_key(left_on, right_on, column);
    let keyed: Vec<PlanComparison> = comparisons
        .iter()
        .filter_map(|comparison| remapped_comparison(comparison, &right_key))
        .collect();
    let mut before = Vec::new();
    let placed_left = place_after(first, &before);
    before.push(filter_of(steps, *left, comparisons.to_vec(), written)?);
    let placed_right = if keyed.is_empty() {
        *right
    } else {
        let at = place_after(first, &before);
        before.push(filter_of(steps, *right, keyed, written)?);
        at
    };
    Some(Below {
        before,
        combined: PlanStep {
            operation: Operation::Antijoin {
                left: placed_left,
                right: placed_right,
                left_on: left_on.clone(),
                right_on: right_on.clone(),
            },
            scope: made.scope,
            written: made.written.clone(),
        },
        moved: comparisons.to_vec(),
    })
}

/// The projection `id` over its input filtered by `comparisons`: a projection keeps columns of the rows it reads, so each is a
/// filter of those rows on the columns it keeps.
fn below_project(
    steps: &[PlanStep],
    id: StepId,
    comparisons: &[PlanComparison],
    first: StepId,
    written: &StepWritten,
) -> Option<Below> {
    let made = step_in(steps, id)?;
    let Operation::Project {
        from,
        columns,
        distinct,
    } = &made.operation
    else {
        return None;
    };
    let read = comparisons
        .iter()
        .map(|comparison| {
            remapped_comparison(comparison, &|column| column_in(columns, column).copied())
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Below {
        before: vec![filter_of(steps, *from, read, written)?],
        combined: PlanStep {
            operation: Operation::Project {
                from: place_after(first, &[]),
                columns: columns.clone(),
                distinct: *distinct,
            },
            scope: made.scope,
            written: made.written.clone(),
        },
        moved: comparisons.to_vec(),
    })
}

/// The extension `id` over its input filtered by those of `comparisons` that test only the columns of the rows it extends.
fn below_extend(
    steps: &[PlanStep],
    id: StepId,
    comparisons: &[PlanComparison],
    first: StepId,
    written: &StepWritten,
) -> Option<Below> {
    let made = step_in(steps, id)?;
    let Operation::Extend { from, values } = &made.operation else {
        return None;
    };
    let read = usize::try_from(width_of(steps, *from)?.0).unwrap_or(usize::MAX);
    let moved: Vec<PlanComparison> = comparisons
        .iter()
        .filter(|comparison| {
            remapped_comparison(comparison, &|column| {
                usize::try_from(column.0)
                    .is_ok_and(|at| at < read)
                    .then_some(column)
            })
            .is_some()
        })
        .cloned()
        .collect();
    if moved.is_empty() {
        return None;
    }
    Some(Below {
        before: vec![filter_of(steps, *from, moved.clone(), written)?],
        combined: PlanStep {
            operation: Operation::Extend {
                from: place_after(first, &[]),
                values: values.clone(),
            },
            scope: made.scope,
            written: made.written.clone(),
        },
        moved,
    })
}

/// The filter `id` with `comparisons` made after its own, as one filter of what it reads (`fuse`).
fn below_select(
    steps: &[PlanStep],
    id: StepId,
    comparisons: &[PlanComparison],
    _first: StepId,
    _written: &StepWritten,
) -> Option<Below> {
    let made = step_in(steps, id)?;
    let reading = Operation::Select {
        from: Input::Step(id),
        comparisons: comparisons.to_vec(),
        kept: Vec::new(),
    };
    Some(Below {
        before: Vec::new(),
        combined: PlanStep {
            operation: fuse(&made.operation, &reading)?,
            scope: made.scope,
            written: made.written.clone(),
        },
        moved: comparisons.to_vec(),
    })
}
