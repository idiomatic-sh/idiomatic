//! The `table_model` service: free functions named by verb over tables, with no IO and no ports.

use std::cmp::Ordering;

use ryo_language::domain::elements::service::real_count;
use ryo_relations::domain::relations::service::from_bits;
use ryo_relations::domain::relations::types::{
    Bits, CellKind, ColumnsView, Flag, Integer, Position, Value,
};

use super::constants::MATERIALIZED;
use super::types::registered_representation::value::RegisteredRepresentation;
use super::types::{RepresentationInput, Rows};
use super::types::RepresentationKind;
use crate::domain::table_cells::service::{edge_of, element_of};
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use ryo_derived::domain::key::service::hash_key;
use ryo_derived::domain::key::types::ChainLink;
use ryo_derived::domain::equality::types::KeyIndex;
use ryo_derived::domain::key::types::On;
use ryo_values::domain::plan_parts::service::column_in;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::table_words::types::TableShape;

use ryo_values::domain::rows::service::{positions, push_row, reset, row_in};

/// What each column of the rows holds.
#[must_use]
pub fn kinds_of<'a>(rows: &'a Rows<'_>) -> &'a [CellKind] {
    match rows {
        Rows::Stored(view) => view.kinds,
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
pub fn count_of(rows: &Rows<'_>) -> RowCount {
    match rows {
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
pub fn read_row<'s>(
    rows: &'s Rows<'s>,
    index: RowIndex,
    scratch: &'s mut Vec<Value>,
) -> Option<&'s [Value]> {
    borrowed(rows).read(index, scratch)
}

/// Copies the rows into `into`, keeping the room `into` has.
pub fn materialize(rows: &Rows<'_>, into: &mut Table, scratch: &mut Vec<Value>) {
    reset(into, kinds_of(rows));
    for index in positions(count_of(rows)) {
        if let Some(row) = read_row(rows, index, scratch) {
            push_row(into, row);
        }
    }
    tracing::trace!(target: "ryo::diagnostic::selection", event = MATERIALIZED,
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
pub fn check_columns(kinds: &[CellKind], columns: &[ColumnIndex]) -> Result<(), RowsError> {
    match columns
        .iter()
        .find(|column| usize::try_from(column.0).map_or(true, |place| place >= kinds.len()))
    {
        Some(missing) => Err(RowsError::NoSuchColumn { index: missing.0 }),
        None => Ok(()),
    }
}

#[cfg(test)]
/// Whether every declared stored column has exactly the first column's row population.
/// Other views keep the materializing path, including its treatment of incomplete rows.
pub(crate) fn rectangular(view: ColumnsView<'_>) -> TableShape {
    let count = view.columns.first().map_or(0, |column| column.0.len());
    if view.columns.len() == view.kinds.len()
        && view.columns.iter().all(|column| column.0.len() == count)
    {
        TableShape::Rectangular
    } else {
        TableShape::Ragged
    }
}

#[cfg(any(test, feature = "fixtures"))]
/// Decode one typed stored cell, preserving normalization such as nonzero flag encodings.
fn stored_cell(view: ColumnsView<'_>, row: RowIndex, column: ColumnIndex) -> Option<Value> {
    let kind = column_in(view.kinds, column)?;
    let bits = column_in(view.columns, column)?.0.get(row.0)?;
    Some(from_bits(*kind, Bits(*bits)))
}

#[cfg(test)]
/// Read one typed cell without decoding unused stored payload or allocating a row buffer.
pub(crate) fn cell_at(rows: &Rows<'_>, row: RowIndex, column: ColumnIndex) -> Option<Value> {
    borrowed(rows)
        .cell(row, column)
}

#[cfg(any(test, feature = "fixtures"))]
/// Find original stored rows by typed keys, checking equality after every hash match.
/// The caller indexes a rectangular view first; payload columns are never read here.
pub fn matching_stored<'o, 'k>(
    view: ColumnsView<'o>,
    columns: &'k [ColumnIndex],
    order: &'o KeyIndex,
    key: &'k [Value],
    #[cfg(feature = "diagnostic-row-progress")]
    observer: &dyn crate::domain::operator::contract::views::ProgressObserver,
) -> impl Iterator<Item = &'o RowIndex> + Clone {
    #[cfg(feature = "diagnostic-row-progress")]
    observer.observe(
        crate::domain::operator::contract::views::ProgressCounter::IndexProbes,
        crate::domain::operator::contract::views::ProgressAmount(1),
    );
    let hash = hash_key(key.iter().map(Some));
    let bucket = usize::try_from(hash.0).unwrap_or(0) & order.heads.len().saturating_sub(1);
    let mut at = order.heads.get(bucket).copied().unwrap_or(ChainLink(0));
    std::iter::from_fn(move || {
        while at.0 != 0 {
            #[cfg(feature = "diagnostic-row-progress")]
            observer.observe(
                crate::domain::operator::contract::views::ProgressCounter::IndexRowsExamined,
                crate::domain::operator::contract::views::ProgressAmount(1),
            );
            let place = usize::try_from(at.0 - 1).unwrap_or(usize::MAX);
            at = order.next.get(place).copied().unwrap_or(ChainLink(0));
            if order.hashes.get(place) == Some(&hash)
                && columns.iter().zip(key).all(|(column, wanted)| {
                    stored_cell(view, RowIndex(place), *column) == Some(*wanted)
                })
            {
                let row = order.rows.get(place);
                #[cfg(feature = "diagnostic-row-progress")]
                if row.is_some() {
                    observer.observe(
                        crate::domain::operator::contract::views::ProgressCounter::EqualityMatches,
                        crate::domain::operator::contract::views::ProgressAmount(1),
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
    let bucket = usize::try_from(hash.0).unwrap_or(0) & order.heads.len().saturating_sub(1);
    // The next row of the chain, plus one; zero when there is none.
    let mut at = order.heads.get(bucket).copied().unwrap_or(ChainLink(0));
    std::iter::from_fn(move || {
        while at.0 != 0 {
            let place = usize::try_from(at.0 - 1).unwrap_or(usize::MAX);
            at = order.next.get(place).copied().unwrap_or(ChainLink(0));
            if order.hashes.get(place) == Some(&hash)
                && row_in(table, RowIndex(place))
                    .is_some_and(|row| compare_key(row, on, key) == Ordering::Equal)
            {
                return order.rows.get(place);
            }
        }
        None
    })
}

use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// The classified catalog value that reads rows of `kind`.
pub fn registered(kind: RepresentationKind) -> RegisteredRepresentation {
    RegisteredRepresentation::new(kind)
}

/// The representation `input` is held in.
pub fn kind(input: RepresentationInput<'_>) -> RepresentationKind {
    RegisteredRepresentation::kind(input)
}

/// The rows as a physical representation reads them.
pub(crate) fn borrowed<'a>(rows: &'a Rows<'a>) -> RepresentationInput<'a> {
    RepresentationInput::from(rows)
}
