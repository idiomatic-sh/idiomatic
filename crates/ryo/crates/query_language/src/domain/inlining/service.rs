// the three cuts of the rows handed over (a step's columns, a relation's columns, a view read in place) share the reading of what an operation reads of an input, which is one match over the operations; splitting it would repeat that match.
//! The `inlining` service: free functions named by verb over the views of a relation read in place, and the rows handed over cut to the columns asked.

use ryo_relations::domain::relations::service::provenance_of;
use ryo_relations::domain::relations::types::{Provenance, Relation};

use super::types::StepReading;
use crate::domain::rewriting::types::Planned;
use ryo_values::domain::plan_parts::service::{each_column, each_column_mut};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::service::each_input;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;
use ryo_values::domain::plan_parts::types::StepScope;

/// The columns a join or an axis names of its input on `side`.
fn side_columns(output: &[JoinColumn], side: Side) -> impl Iterator<Item = ColumnIndex> + '_ {
    output
        .iter()
        .filter(move |each| each.side == side)
        .map(|each| each.column)
}

/// The columns a join or an axis names of its input on `side`, to be renamed.
fn side_columns_mut(
    output: &mut [JoinColumn],
    side: Side,
) -> impl Iterator<Item = &mut ColumnIndex> + '_ {
    output
        .iter_mut()
        .filter(move |each| each.side == side)
        .map(|each| &mut each.column)
}

/// The columns the comparisons read, those the arguments of a function read among them.
fn compared_columns(comparisons: &[PlanComparison]) -> impl Iterator<Item = ColumnIndex> + '_ {
    let mut read = Vec::new();
    for comparison in comparisons {
        each_column(&comparison.left, &mut |column| read.push(column));
        each_column(&comparison.right, &mut |column| read.push(column));
    }
    read.into_iter()
}

/// Renames each column the comparisons read with `place`, those the arguments of a function read among them.
fn rename_compared<F>(comparisons: &mut [PlanComparison], mut place: F)
where
    F: FnMut(&mut ColumnIndex),
{
    for comparison in comparisons {
        each_column_mut(&mut comparison.left, &mut place);
        each_column_mut(&mut comparison.right, &mut place);
    }
}

/// Columns demanded from one of a join's inputs, retaining side and output order.
fn join_read_columns(
    left: Input,
    right: Input,
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
    output: &[JoinColumn],
    read: Input,
) -> Option<Vec<ColumnIndex>> {
    if left != read && right != read {
        return None;
    }
    let mut used = Vec::new();
    if left == read {
        used.extend(left_on);
        used.extend(side_columns(output, Side::Left));
    }
    if right == read {
        used.extend(right_on);
        used.extend(side_columns(output, Side::Right));
    }
    Some(used)
}

/// The columns of the step `read` that the operation reads, when it reads only those and says so by what it keeps
/// of them; nothing when it reads them all, or in a way this cannot cut.
fn reading_columns(operation: &Operation, read: Input) -> Option<Vec<ColumnIndex>> {
    let is = |input: &Input| *input == read;
    let mut used: Vec<ColumnIndex> = Vec::new();
    match operation {
        Operation::Select {
            from,
            comparisons,
            kept,
        } if is(from) && !kept.is_empty() => {
            used.extend(kept);
            used.extend(compared_columns(comparisons));
        }
        Operation::Project { from, columns, .. } if is(from) => used.extend(columns),
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
            if is(right) && (!filter.is_empty() || !view.is_empty()) {
                return None;
            }
            used.extend(join_read_columns(
                *left, *right, left_on, right_on, output, read,
            )?);
        }
        Operation::Antijoin {
            left,
            right,
            right_on,
            ..
        } if is(right) && !is(left) => used.extend(right_on),
        Operation::GroupCount {
            from,
            keys,
            distinct,
        } if is(from) => {
            used.extend(keys);
            used.extend(distinct);
        }
        // An axis reads the starting rows by their id and the columns its answer names of them, the edges by their parent and child, and
        // the elements that prove an answer valid by their id, the columns their filter reads and the columns its answer names of them.
        Operation::Axis {
            from,
            edges,
            domain,
            id,
            parent,
            child,
            domain_id,
            domain_filter,
            domain_view,
            output,
            ..
        } if (is(from) || is(edges) || is(domain))
            && domain_view.is_empty()
            && (!output.is_empty() || !is(from)) =>
        {
            if is(from) {
                used.push(*id);
                used.extend(side_columns(output, Side::Left));
            }
            if is(edges) {
                used.extend([*parent, *child]);
            }
            if is(domain) {
                used.push(*domain_id);
                used.extend(compared_columns(domain_filter));
                used.extend(side_columns(output, Side::Right));
            }
        }
        Operation::Select { .. }
        | Operation::Project { .. }
        | Operation::Extend { .. }
        // Demand RHS keys address a virtual closure schema, not the physical edge columns.
        // This earlier column-remapping pass must leave that physical descriptor intact.
        | Operation::Join { reach: Some(_), .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
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
        | Operation::Words { .. }
        | Operation::Rank { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => return None,
    }
    Some(used)
}

/// Canonical distinct column indices needed from one input of an operation.
fn columns_read(operation: &Operation, read: Input) -> Option<Vec<ColumnIndex>> {
    let mut used = reading_columns(operation, read)?;
    used.sort();
    used.dedup();
    Some(used)
}

/// The operation reading the step `read` now reading `through` instead, each column it names there named by `map`.
fn reading_through(
    operation: &mut Operation,
    read: Input,
    through: Input,
    map: &dyn Fn(ColumnIndex) -> ColumnIndex,
) {
    let place = |column: &mut ColumnIndex| *column = map(*column);
    let is = |input: &Input| *input == read;
    match operation {
        Operation::Select {
            from,
            comparisons,
            kept,
        } if is(from) => {
            *from = through;
            kept.iter_mut().for_each(place);
            rename_compared(comparisons, place);
        }
        Operation::Project { from, columns, .. } if is(from) => {
            *from = through;
            columns.iter_mut().for_each(place);
        }
        Operation::Join {
            left,
            right,
            left_on,
            right_on,
            output,
            ..
        } => {
            if is(left) {
                *left = through;
                left_on.iter_mut().for_each(place);
                side_columns_mut(output, Side::Left).for_each(place);
            }
            if is(right) {
                *right = through;
                right_on.iter_mut().for_each(place);
                side_columns_mut(output, Side::Right).for_each(place);
            }
        }
        Operation::Antijoin {
            right, right_on, ..
        } if is(right) => {
            *right = through;
            right_on.iter_mut().for_each(place);
        }
        Operation::GroupCount {
            from,
            keys,
            distinct,
        } if is(from) => {
            *from = through;
            keys.iter_mut().for_each(place);
            distinct.iter_mut().for_each(place);
        }
        Operation::Axis {
            from,
            edges,
            domain,
            id,
            parent,
            child,
            domain_id,
            domain_filter,
            output,
            ..
        } => {
            if is(from) {
                *from = through;
                place(id);
                side_columns_mut(output, Side::Left).for_each(place);
            }
            if is(edges) {
                *edges = through;
                place(parent);
                place(child);
            }
            if is(domain) {
                *domain = through;
                place(domain_id);
                rename_compared(domain_filter, place);
                side_columns_mut(output, Side::Right).for_each(place);
            }
        }
        Operation::Select { .. }
        | Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
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
        | Operation::Words { .. }
        | Operation::Rank { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => {}
    }
}

/// The steps with each filter-free view of a relation that only reorders its columns read in place by the per-file steps
/// that read it: they ask the relation for the columns they name, and no table of the whole relation is made.
#[must_use]
pub(crate) fn inline_views(mut steps: Vec<PlanStep>) -> Vec<PlanStep> {
    for view in 0..steps.len() {
        let Some(viewed) = steps.get(view) else {
            continue;
        };
        let scope = viewed.scope;
        let Operation::Select {
            from: from @ Input::Relation(Relation::SyntaxChild | Relation::SyntaxTree),
            comparisons,
            kept,
        } = viewed.operation.clone()
        else {
            continue;
        };
        if !comparisons.is_empty() || kept.is_empty() || scope != StepScope::PerFile {
            continue;
        }
        let read = StepId(u32::try_from(view).unwrap_or(u32::MAX));
        for step in steps.iter_mut().skip(view + 1) {
            if step.scope != StepScope::PerFile
                || columns_read(&step.operation, Input::Step(read)).is_none()
            {
                continue;
            }
            reading_through(&mut step.operation, Input::Step(read), from, &|column| {
                let at = usize::try_from(column.0).unwrap_or(usize::MAX);
                kept.get(at).copied().unwrap_or(column)
            });
        }
    }
    steps
}

/// Resolve an original column to its place in the reduced frontier projection.
fn projection_column(used: &[ColumnIndex], column: ColumnIndex) -> ColumnIndex {
    used.iter()
        .position(|each| *each == column)
        .map_or(column, |at| {
            ColumnIndex(u32::try_from(at).unwrap_or(u32::MAX))
        })
}

/// The plan with, for each per-file step that only cross-file steps read through some of its columns, one projection
/// of those columns without repeats: the rows a file hands over are the distinct ones the cross-file steps ask for, not
/// one for each place in the file. Not done when a count of rows (not of distinct values) comes after the files, which
/// would count the repeats.
#[must_use]
pub(crate) fn reduced_frontier(steps: Vec<PlanStep>, mut emitted: StepId) -> Planned {
    let steps = reduced_frontier_roots(steps, std::slice::from_mut(&mut emitted));
    Planned { steps, emitted }
}

/// The same rewrite with every externally observable root protected and renumbered together.
pub(crate) fn reduced_frontier_roots(
    mut steps: Vec<PlanStep>,
    roots: &mut [StepId],
) -> Vec<PlanStep> {
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
