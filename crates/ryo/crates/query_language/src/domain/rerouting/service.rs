//! The `rerouting` service: an operation that reads one step made to read another in its place, each column it names there
//! renamed.

use ryo_values::domain::band_joins::types::BandJoin;
use ryo_values::domain::plan_parts::service::each_column_mut;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plans::types::Operation;
use std::cell::Cell;

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

/// The operation reading the step `read` now reading `through` instead, each column it names there named by `map`; nothing when
/// `map` has no column for one of them, and then the operation may be left part renamed: a caller that can be refused renames a copy.
pub(crate) fn reading_through(
    operation: &mut Operation,
    read: Input,
    through: Input,
    map: &dyn Fn(ColumnIndex) -> Option<ColumnIndex>,
) -> Option<()> {
    let unmapped = Cell::new(false);
    let place = |column: &mut ColumnIndex| match map(*column) {
        Some(mapped) => *column = mapped,
        None => unmapped.set(true),
    };
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
        Operation::BandJoin(BandJoin {
            left,
            right,
            left_on,
            right_on,
            band,
            output,
            ..
        }) => {
            if is(left) {
                *left = through;
                left_on.iter_mut().for_each(place);
                band.iter_mut().for_each(|edge| place(&mut edge.left));
                side_columns_mut(output, Side::Left).for_each(place);
            }
            if is(right) {
                *right = through;
                right_on.iter_mut().for_each(place);
                band.iter_mut().for_each(|edge| place(&mut edge.right));
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
        | Operation::Worklist { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Words { .. }
        | Operation::Rank { .. }
        | Operation::Closure { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. } => {}
    }
    (!unmapped.get()).then_some(())
}
