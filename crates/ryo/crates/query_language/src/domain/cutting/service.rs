//! The `cutting` service: free functions named by verb over the rows a file hands over, cut to the columns the steps after the files ask.

use crate::domain::inlining::service::columns_read;
use crate::domain::inlining::service::projection_column;
use crate::domain::inlining::service::reading_through;
use crate::domain::inlining::types::StepReading;
use crate::domain::narrowing::service::{narrow_lookups, narrow_lookups_roots};
use crate::domain::regrouping::service::{regroup_joins, regroup_joins_roots};
use crate::domain::rewriting::types::Planned;
use ryo_relations::domain::relations::service::provenance_of;
use ryo_relations::domain::relations::types::Provenance;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::service::each_input;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

/// The plan with the rows a file hands over made as few as the steps after the files ask: the per-file inputs of each join after the
/// files joined in each file, each per-file input of a join after the files narrowed to the rows its key can meet, then each per-file
/// step cut to the distinct rows of the columns asked of it, and each relation to its columns asked.
#[must_use]
pub(crate) fn cut_handover(steps: Vec<PlanStep>, emitted: StepId) -> Planned {
    let Planned { steps, emitted } = regroup_joins(steps, emitted);
    let Planned { steps, emitted } = narrow_lookups(steps, emitted);
    let Planned { steps, emitted } = reduced_frontier(steps, emitted);
    cut_relations(steps, emitted)
}

/// The same rewrites with every externally observable root protected and renumbered together.
pub(crate) fn cut_handover_roots(steps: Vec<PlanStep>, roots: &mut [StepId]) -> Vec<PlanStep> {
    let steps = regroup_joins_roots(steps, roots);
    let steps = narrow_lookups_roots(steps, roots);
    let steps = reduced_frontier_roots(steps, roots);
    cut_relations_roots(steps, roots)
}

/// The plan with, for each per-file step that only cross-file steps read through some of its columns, one projection
/// of those columns without repeats: the rows a file hands over are the distinct ones the cross-file steps ask for, not
/// one for each place in the file. Not done when a count of rows (not of distinct values) comes after the files, which
/// would count the repeats.
#[must_use]
fn reduced_frontier(steps: Vec<PlanStep>, mut emitted: StepId) -> Planned {
    let steps = reduced_frontier_roots(steps, std::slice::from_mut(&mut emitted));
    Planned { steps, emitted }
}

/// The same rewrite with every externally observable root protected and renumbered together.
fn reduced_frontier_roots(mut steps: Vec<PlanStep>, roots: &mut [StepId]) -> Vec<PlanStep> {
    if steps.iter().any(|step| {
        step.scope == StepScope::CrossFile
            && matches!(step.operation, Operation::GroupCount { distinct: None, .. })
    }) {
        return steps;
    }
    for candidate in (0..steps.len()).rev() {
        let read = StepId(u32::try_from(candidate).unwrap_or(u32::MAX));
        let Some(cut_step) = steps.get(candidate) else {
            continue;
        };
        if cut_step.scope != StepScope::PerFile || roots.contains(&read) {
            continue;
        }
        let consumers: Vec<usize> = steps
            .iter()
            .enumerate()
            .filter(|(_, step)| {
                step.scope == StepScope::CrossFile
                    && operation_reads(&step.operation, read) == StepReading::Reads
            })
            .map(|(place, _)| place)
            .collect();
        if consumers.is_empty() {
            continue;
        }
        let mut used: Vec<ColumnIndex> = Vec::new();
        let mut cut = true;
        for step in consumers.iter().filter_map(|place| steps.get(*place)) {
            match columns_read(&step.operation, Input::Step(read)) {
                Some(columns) => used.extend(columns),
                None => cut = false,
            }
        }
        used.sort();
        used.dedup();
        // Rows a join or a count made are each of something else; those a filter, a capture or a split made repeat
        // wherever the file does, and a step that makes only the columns asked of it already is the reduced rows.
        let made = if let Operation::Select { kept, .. } = &cut_step.operation
            && !kept.is_empty()
        {
            Some(kept.len())
        } else {
            None
        };
        let repeats = matches!(
            cut_step.operation,
            Operation::Select { .. }
                | Operation::Capture { .. }
                | Operation::Words { .. }
                | Operation::Extend { .. }
                | Operation::SpanBounds { .. }
                | Operation::SourceMatches { .. }
                | Operation::RenderSyntax { .. }
                | Operation::FitsRendered { .. }
                | Operation::SpanBetween { .. }
                | Operation::Project {
                    distinct: RepeatedRows::Kept,
                    ..
                }
        );
        if !cut || used.is_empty() || !repeats || made == Some(used.len()) {
            continue;
        }
        // The projection follows the step it cuts, so every step reads only what comes before it.
        let through = StepId(read.0 + 1);
        for step in &mut steps {
            each_input(&mut step.operation, &mut |input| {
                if let Input::Step(id) = input
                    && id.0 > read.0
                {
                    id.0 += 1;
                }
            });
        }
        for root in roots.iter_mut() {
            if root.0 > read.0 {
                root.0 += 1;
            }
        }
        steps.insert(
            candidate + 1,
            PlanStep {
                operation: Operation::Project {
                    from: Input::Step(read),
                    columns: used.clone(),
                    distinct: RepeatedRows::Dropped,
                },
                scope: StepScope::PerFile,
            },
        );
        for place in consumers {
            // The step inserted before it moved each consumer one place on.
            let Some(consumer) = steps.get_mut(place + 1) else {
                continue;
            };
            reading_through(
                &mut consumer.operation,
                Input::Step(read),
                Input::Step(through),
                &|column| projection_column(&used, column),
            );
        }
    }
    steps
}

/// Whether the operation reads the step `read`.
fn operation_reads(operation: &Operation, read: StepId) -> StepReading {
    let mut reads = false;
    let mut copy = operation.clone();
    each_input(&mut copy, &mut |input| reads |= *input == Input::Step(read));
    if reads {
        StepReading::Reads
    } else {
        StepReading::Ignores
    }
}

/// The plan with, for each relation a cross-file step reads whole, one step that cuts it to the columns the cross-file steps ask of it:
/// a file hands over those columns of its rows, and not the others. A relation read for all its columns is left as it is.
#[must_use]
pub(crate) fn cut_relations(steps: Vec<PlanStep>, mut emitted: StepId) -> Planned {
    let steps = cut_relations_roots(steps, std::slice::from_mut(&mut emitted));
    Planned { steps, emitted }
}

/// The same rewrite with every externally observable root protected and renumbered together.
pub(crate) fn cut_relations_roots(mut steps: Vec<PlanStep>, roots: &mut [StepId]) -> Vec<PlanStep> {
    let mut wanted: Vec<Relation> = Vec::new();
    for step in steps
        .iter()
        .filter(|step| step.scope == StepScope::CrossFile)
    {
        let mut copy = step.operation.clone();
        each_input(&mut copy, &mut |input| {
            if let Input::Relation(relation) = input
                && !wanted.contains(relation)
                && provenance_of(*relation) != Provenance::Manifests
            {
                wanted.push(*relation);
            }
        });
    }
    for relation in wanted {
        let read = Input::Relation(relation);
        let consumers: Vec<usize> = steps
            .iter()
            .enumerate()
            .filter(|(_, step)| {
                step.scope == StepScope::CrossFile && {
                    let mut reads = false;
                    let mut copy = step.operation.clone();
                    each_input(&mut copy, &mut |input| reads |= *input == read);
                    reads
                }
            })
            .map(|(place, _)| place)
            .collect();
        let mut used: Vec<ColumnIndex> = Vec::new();
        let mut cut = !consumers.is_empty();
        for step in consumers.iter().filter_map(|place| steps.get(*place)) {
            match columns_read(&step.operation, read) {
                Some(columns) => used.extend(columns),
                None => cut = false,
            }
        }
        used.sort();
        used.dedup();
        if !cut
            || used.is_empty()
            || used.len() >= ryo_relations::domain::relations::service::columns(relation).len()
        {
            continue;
        }
        // The cut comes first, so every step reads only what comes before it.
        for step in &mut steps {
            each_input(&mut step.operation, &mut |input| {
                if let Input::Step(id) = input {
                    id.0 += 1;
                }
            });
        }
        for root in roots.iter_mut() {
            root.0 += 1;
        }
        steps.insert(
            0,
            PlanStep {
                operation: Operation::Select {
                    from: read,
                    comparisons: Vec::new(),
                    kept: used.clone(),
                },
                scope: StepScope::PerFile,
            },
        );
        for place in consumers {
            // The step inserted before it moved each consumer one place on.
            let Some(consumer) = steps.get_mut(place + 1) else {
                continue;
            };
            reading_through(
                &mut consumer.operation,
                read,
                Input::Step(StepId(0)),
                &|column| projection_column(&used, column),
            );
        }
    }
    steps
}
