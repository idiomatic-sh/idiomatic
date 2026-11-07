//! The `table_model` service: free functions named by verb over tables, with no IO and no ports.

use std::cmp::Ordering;

#[cfg(test)]
thread_local! {
    /// Actual full-row reads for same-binary kernel differential tests only.
    static READ_ROW_COUNTS: std::cell::Cell<(u64, u64)> = const { std::cell::Cell::new((0, 0)) };
}

#[cfg(test)]
pub(crate) fn read_row_counts() -> (u64, u64) {
    READ_ROW_COUNTS.with(std::cell::Cell::get)
}

#[cfg(test)]
pub(crate) fn take_read_row_counts() -> (u64, u64) {
    READ_ROW_COUNTS.with(|counts| counts.replace((0, 0)))
}

use ryo_language::domain::elements::service::real_count;
use ryo_relations::domain::relations::service::from_bits;
use ryo_relations::domain::relations::types::{
    Bits, CellKind, ColumnsView, Flag, Integer, Position, Value,
};

use super::types::Rows;
use crate::domain::table_cells::service::{edge_of, element_of};
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use ryo_derived::domain::contract::key::hash_key;
use ryo_derived::domain::contract::types::{ChainLink, KeyIndex, On};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::errors::RowsError;

use ryo_values::domain::rows::service::{positions, push_row, reset, row_in};

/// What each column of the rows holds.
#[must_use]
pub(crate) fn kinds_of<'a>(rows: &'a Rows<'_>) -> &'a [CellKind] {
    match rows {
        Rows::Stored(view) => view.kinds,
        Rows::Selected(view) => &view.selection.kinds,
        Rows::Positioned(view) => view.kinds,
        Rows::Tree(TreeRows {
            form: TreeForm::Edges,
            ..
        }) => &EDGE_KINDS,
        Rows::Tree(TreeRows {
            form: TreeForm::Elements,
            ..
        }) => &ELEMENT_KINDS,
        Rows::Computed(table) => &table.kinds,
        Rows::Compact(table) => ryo_values::domain::compact_table::service::compact_kinds(table),
    }
}

/// How many rows there are.
#[must_use]
pub(crate) fn count_of(rows: &Rows<'_>) -> RowCount {
    match rows {
        Rows::Selected(view) => RowCount(view.selection.rows.len()),
        Rows::Positioned(view) => RowCount(
            view.positions
                .slots
                .0
                .len()
                .checked_div(view.positions.width.0)
                .unwrap_or(0),
        ),
        Rows::Stored(view) => RowCount(view.columns.first().map_or(0, |column| column.0.len())),
        Rows::Computed(table) => table.count,
        Rows::Compact(table) => ryo_values::domain::compact_table::service::compact_rows(table),
        Rows::Tree(tree) => RowCount(
            real_count(tree.elements)
                .0
                .saturating_sub(usize::from(tree.form == TreeForm::Edges)),
        ),
    }
}

/// What the columns of the edges of a tree hold: the parent, the child, the child's place under
/// its parent, and the file.
const EDGE_KINDS: [CellKind; 5] = [
    CellKind::Node,
    CellKind::Node,
    CellKind::Number,
    CellKind::Symbol,
    CellKind::File,
];

/// What the columns of the elements of a tree hold: the element, its kind, its text, the file, and where it is.
const ELEMENT_KINDS: [CellKind; 5] = [
    CellKind::Node,
    CellKind::Symbol,
    CellKind::Symbol,
    CellKind::File,
    CellKind::Span,
];

/// The row at `index`: a computed row where it is, a stored row read into `scratch`, which keeps
/// its room from row to row.
pub(crate) fn read_row<'s>(
    rows: &'s Rows<'s>,
    index: RowIndex,
    scratch: &'s mut Vec<Value>,
) -> Option<&'s [Value]> {
    let found = crate::domain::table_model::collections::representations::service::borrowed(rows)
        .read(index, scratch);
    #[cfg(test)]
    if let Some(row) = found {
        READ_ROW_COUNTS.with(|counts| {
            let (calls, cells) = counts.get();
            counts.set((calls + 1, cells + row.len() as u64));
        });
    }
    found
}

/// Copies the rows into `into`, keeping the room `into` has.
pub(crate) fn materialize(rows: &Rows<'_>, into: &mut Table, scratch: &mut Vec<Value>) {
    reset(into, kinds_of(rows));
    if let Rows::Selected(view) = rows {
        // The selection already names physical positions and projected columns.
        // Decode only those cells into the destination; a malformed source row
        // remains skipped as it was by read_row, without retaining a partial row.
        for row in positions(count_of(rows)) {
            let start = into.cells.len();
            let complete = view.selection.columns.iter().enumerate().all(|(at, _)| {
                let Ok(at) = u32::try_from(at) else {
                    return false;
                };
                let Some(value) = selected_cell(view, row, ColumnIndex(at)) else {
                    return false;
                };
                into.cells.push(value);
                true
            });
            if complete {
                into.count.0 += 1;
            } else {
                into.cells.truncate(start);
            }
        }
    } else {
        for index in positions(count_of(rows)) {
            if let Some(row) = read_row(rows, index, scratch) {
                push_row(into, row);
            }
        }
    }
    tracing::trace!(target: "ryo::diagnostic::selection", event = "materialized",
        compact_source = matches!(rows, Rows::Compact(_)),
        rows = into.count.0, columns = into.kinds.len(),
        cell_bytes = into.cells.len().saturating_mul(size_of::<ryo_relations::domain::relations::types::Value>()),
        capacity_bytes = into.cells.capacity().saturating_mul(size_of::<ryo_relations::domain::relations::types::Value>()));
}

/// How the row compares with `key`, by the cells `on` names.
pub(crate) fn compare_key(row: &[Value], on: On<'_>, key: &[Value]) -> Ordering {
    match on {
        On::Whole => row.cmp(key),
        On::Columns(columns) => {
            for (column, wanted) in columns.iter().zip(key) {
                match column_in(row, *column).map(|each| each.cmp(wanted)) {
                    Some(Ordering::Equal) => {}
                    Some(other) => return other,
                    None => return Ordering::Less,
                }
            }
            Ordering::Equal
        }
    }
}

/// Whether a table whose columns hold `kinds` has every one of `columns`.
///
/// # Errors
///
/// `NoSuchColumn` naming the first it lacks.
pub(crate) fn check_columns(kinds: &[CellKind], columns: &[ColumnIndex]) -> Result<(), RowsError> {
    match columns
        .iter()
        .find(|column| usize::try_from(column.0).map_or(true, |place| place >= kinds.len()))
    {
        Some(missing) => Err(RowsError::NoSuchColumn { index: missing.0 }),
        None => Ok(()),
    }
}

/// Whether every declared stored column has exactly the first column's row population.
/// Other views keep the materializing path, including its treatment of incomplete rows.
pub(crate) fn rectangular(view: ColumnsView<'_>) -> bool {
    let count = view.columns.first().map_or(0, |column| column.0.len());
    view.columns.len() == view.kinds.len()
        && view.columns.iter().all(|column| column.0.len() == count)
}

/// Decode one typed stored cell, preserving normalization such as nonzero flag encodings.
fn stored_cell(view: ColumnsView<'_>, row: RowIndex, column: ColumnIndex) -> Option<Value> {
    let place = column.0 as usize;
    let kind = view.kinds.get(place)?;
    let bits = view.columns.get(place)?.0.get(row.0)?;
    Some(from_bits(*kind, Bits(*bits)))
}

/// Decode one logical compact-view cell directly from its physical provider column.
fn selected_cell(
    view: &super::types::SelectedRows<'_>,
    row: RowIndex,
    column: ColumnIndex,
) -> Option<Value> {
    selected_view_cell(
        super::types::SelectedView {
            source: view.source,
            positions: &view.selection.rows,
            columns: &view.selection.columns,
        },
        row,
        column,
    )
}

fn selected_view_cell(
    view: super::types::SelectedView<'_>,
    row: RowIndex,
    column: ColumnIndex,
) -> Option<Value> {
    let physical = RowIndex(*view.positions.get(row.0)? as usize);
    stored_cell(view.source, physical, *view.columns.get(column.0 as usize)?)
}

/// Read one typed cell without decoding unused stored payload or allocating a row buffer.
pub(crate) fn cell_at(rows: &Rows<'_>, row: RowIndex, column: ColumnIndex) -> Option<Value> {
    crate::domain::table_model::collections::representations::service::borrowed(rows)
        .cell(row, column)
}

/// Find original stored rows by typed keys, checking equality after every hash match.
/// The caller indexes a rectangular view first; payload columns are never read here.
pub(crate) fn matching_stored<'o, 'k>(
    view: ColumnsView<'o>,
    columns: &'k [ColumnIndex],
    order: &'o KeyIndex,
    key: &'k [Value],
    #[cfg(feature = "diagnostic-row-progress")]
    observer: crate::domain::operator::contract::views::ProgressObserver,
) -> impl Iterator<Item = &'o RowIndex> + Clone {
    #[cfg(feature = "diagnostic-row-progress")]
    observer(
        crate::domain::operator::contract::views::ProgressCounter::IndexProbes,
        1,
    );
    let hash = hash_key(key.iter().map(Some));
    let bucket = usize::try_from(hash.0).unwrap_or(0) & order.heads().len().saturating_sub(1);
    let mut at = order.heads().get(bucket).copied().unwrap_or(ChainLink(0));
    std::iter::from_fn(move || {
        while at.0 != 0 {
            #[cfg(feature = "diagnostic-row-progress")]
            observer(
                crate::domain::operator::contract::views::ProgressCounter::IndexRowsExamined,
                1,
            );
            let place = usize::try_from(at.0 - 1).unwrap_or(usize::MAX);
            at = order.links().get(place).copied().unwrap_or(ChainLink(0));
            if order.hashes().get(place) == Some(&hash)
                && columns.iter().zip(key).all(|(column, wanted)| {
                    stored_cell(view, RowIndex(place), *column) == Some(*wanted)
                })
            {
                let row = order.rows().get(place);
                #[cfg(feature = "diagnostic-row-progress")]
                if row.is_some() {
                    observer(
                        crate::domain::operator::contract::views::ProgressCounter::EqualityMatches,
                        1,
                    );
                }
                return row;
            }
        }
        None
    })
}

/// The rows of an indexed table whose keys are `key`, in the order they came: the chain of the key's bucket is followed, and a row
/// whose hash or key differs is passed over.
pub(crate) fn matching<'o, 'k>(
    table: &'o Table,
    on: On<'k>,
    order: &'o KeyIndex,
    key: &'k [Value],
) -> impl Iterator<Item = &'o RowIndex> + Clone {
    let hash = hash_key(key.iter().map(Some));
    let bucket = usize::try_from(hash.0).unwrap_or(0) & order.heads().len().saturating_sub(1);
    // The next row of the chain, plus one; zero when there is none.
    let mut at = order.heads().get(bucket).copied().unwrap_or(ChainLink(0));
    std::iter::from_fn(move || {
        while at.0 != 0 {
            let place = usize::try_from(at.0 - 1).unwrap_or(usize::MAX);
            at = order.links().get(place).copied().unwrap_or(ChainLink(0));
            if order.hashes().get(place) == Some(&hash)
                && row_in(table, RowIndex(place))
                    .is_some_and(|row| compare_key(row, on, key) == Ordering::Equal)
            {
                return order.rows().get(place);
            }
        }
        None
    })
}

use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

use ryo_values::domain::rows::service::column_in;
