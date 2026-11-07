//! The `inlining` service: free functions named by verb over the views of a relation read in place, and the rows handed over cut to the columns asked.

use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::service::each_column;
use ryo_values::domain::plan_parts::service::each_column_mut;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::PlanStep;

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
pub(crate) fn columns_read(operation: &Operation, read: Input) -> Option<Vec<ColumnIndex>> {
    let mut used = reading_columns(operation, read)?;
    used.sort();
    used.dedup();
    Some(used)
}

/// The operation reading the step `read` now reading `through` instead, each column it names there named by `map`.
pub(crate) fn reading_through(
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
pub(crate) fn projection_column(used: &[ColumnIndex], column: ColumnIndex) -> ColumnIndex {
    used.iter()
        .position(|each| *each == column)
        .map_or(column, |at| {
            ColumnIndex(u32::try_from(at).unwrap_or(u32::MAX))
        })
}
