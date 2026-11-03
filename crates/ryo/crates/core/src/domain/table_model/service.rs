//! The `table_model` service: free functions named by verb over tables, with no IO and no ports.

use std::cmp::Ordering;

#[cfg(test)]
thread_local! {
    /// Actual full-row reads for same-binary kernel differential tests only.
    static READ_ROW_COUNTS: std::cell::Cell<(u64, u64)> = const { std::cell::Cell::new((0, 0)) };
}

#[cfg(test)]
pub(crate) fn take_read_row_counts() -> (u64, u64) {
    READ_ROW_COUNTS.with(|counts| counts.replace((0, 0)))
}

use ryo_language::domain::elements::service::{
    has_incomplete_syntax, kind_names, real_count, syntax_acceptance,
};
use ryo_language::domain::elements::types::SyntaxAcceptance;
use ryo_relations::domain::relations::service::{from_bits, kinds};
use ryo_relations::domain::relations::types::{
    Bits, CellKind, ColumnsView, Flag, Integer, Position, Relation, Value,
};

use super::errors::TableModelError;
use super::types::{ChainLink, HashWord, KeyCellsHash, KeyIndex, On, RowCount, Rows, Table};
use crate::domain::coverage::types::Partial;
use crate::domain::table_cells::service::{edge_of, element_of};
use crate::domain::table_cells::types::{RowIndex, TreeForm, TreeRows};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The one of `items` that the column `index` names, when there is one: the one conversion between a column number and a place.
#[must_use]
pub(crate) fn column_in<T>(items: &[T], index: ColumnIndex) -> Option<&T> {
    items.get(usize::try_from(index.0).unwrap_or(usize::MAX))
}

/// The value of the column at `index` in `row`.
///
/// # Errors
///
/// `NoSuchColumn` when the row is shorter.
pub(crate) fn cell(row: &[Value], index: ColumnIndex) -> Result<Value, TableModelError> {
    column_in(row, index)
        .copied()
        .ok_or(TableModelError::NoSuchColumn { index: index.0 })
}

/// The row at `index`, when the table has that many rows.
#[must_use]
pub(crate) fn row_in(table: &Table, index: RowIndex) -> Option<&[Value]> {
    let (width, index) = (table.kinds.len(), index.0);
    table
        .cells
        .get(index * width..(index + 1) * width)
        .filter(|_| index < table.count.0)
}

/// The places of rows, from the first to the last of that many.
pub(crate) fn positions(count: RowCount) -> impl Iterator<Item = RowIndex> + Clone {
    (0..count.0).map(RowIndex)
}

/// Empties the table for new rows of the given kinds, keeping its room.
pub(crate) fn reset(table: &mut Table, kinds: &[CellKind]) {
    table.kinds.clear();
    table.kinds.extend_from_slice(kinds);
    table.cells.clear();
    table.count = RowCount(0);
}

/// Adds one row to the table.
pub(crate) fn push_row(table: &mut Table, row: &[Value]) {
    table.cells.extend_from_slice(row);
    table.count = RowCount(table.count.0 + 1);
}

/// Makes `into` a copy of `from`, keeping the room `into` has.
pub(crate) fn copy_table(from: &Table, into: &mut Table) {
    reset(into, &from.kinds);
    into.cells.extend_from_slice(&from.cells);
    into.count = from.count;
}

/// What each column of the rows holds.
#[must_use]
pub(crate) fn kinds_of<'a>(rows: &'a Rows<'_>) -> &'a [CellKind] {
    match rows {
        Rows::Stored(view) => view.kinds,
        Rows::Selected(view) => &view.selection.kinds,
        Rows::Tree(TreeRows {
            form: TreeForm::Edges,
            ..
        }) => &EDGE_KINDS,
        Rows::Tree(TreeRows {
            form: TreeForm::Elements,
            ..
        }) => &ELEMENT_KINDS,
        Rows::Computed(table) => &table.kinds,
    }
}

/// How many rows there are.
#[must_use]
pub(crate) fn count_of(rows: &Rows<'_>) -> RowCount {
    match rows {
        Rows::Selected(view) => RowCount(view.selection.rows.len()),
        Rows::Stored(view) => RowCount(view.columns.first().map_or(0, |column| column.0.len())),
        Rows::Computed(table) => table.count,
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
        rows = into.count.0, columns = into.kinds.len(),
        cell_bytes = into.cells.len().saturating_mul(size_of::<ryo_relations::domain::relations::types::Value>()),
        capacity_bytes = into.cells.capacity().saturating_mul(size_of::<ryo_relations::domain::relations::types::Value>()));
}

/// The cells of `row` that `on` names, written into `key`, whose room is reused.
///
/// # Errors
///
/// `NoSuchColumn` when the row lacks one of the columns.
pub(crate) fn key_of(
    row: &[Value],
    on: On<'_>,
    key: &mut Vec<Value>,
) -> Result<(), TableModelError> {
    key.clear();
    match on {
        On::Whole => key.extend_from_slice(row),
        On::Columns(columns) => {
            for column in columns {
                key.push(cell(row, *column)?);
            }
        }
    }
    Ok(())
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
pub(crate) fn check_columns(
    kinds: &[CellKind],
    columns: &[ColumnIndex],
) -> Result<(), TableModelError> {
    match columns
        .iter()
        .find(|column| usize::try_from(column.0).map_or(true, |place| place >= kinds.len()))
    {
        Some(missing) => Err(TableModelError::NoSuchColumn { index: missing.0 }),
        None => Ok(()),
    }
}

/// The number a value is for hashing: what it is stored as, apart from which kind of value it is.
pub(crate) fn word_of(value: &Value) -> HashWord {
    HashWord(match value {
        Value::Node(node) => node.0,
        Value::Symbol(symbol) => u64::from(symbol.0),
        Value::Name(name) => name.hash.0,
        Value::File(file) => u64::from(file.0),
        Value::Span(span) => (u64::from(span.start.0) << 32) | u64::from(span.end.0),
        Value::Number(Integer(number)) => number.cast_unsigned(),
        Value::Flag(flag) => u64::from(*flag == Flag::Raised),
    })
}

/// One step of the hash of a run of values.
pub(crate) const fn mix(hash: KeyCellsHash, word: HashWord) -> KeyCellsHash {
    KeyCellsHash((hash.0.rotate_left(5) ^ word.0).wrapping_mul(0x517c_c1b7_2722_0a95))
}

/// The hash of the cells that make a key.
pub(crate) fn hash_key<'v>(cells: impl Iterator<Item = Option<&'v Value>>) -> KeyCellsHash {
    cells.fold(KeyCellsHash(0), |hash, cell| {
        mix(hash, cell.map_or(HashWord(0), word_of))
    })
}

/// The hash of the key a row has.
pub(crate) fn hash_row(row: &[Value], on: On<'_>) -> KeyCellsHash {
    match on {
        On::Whole => hash_key(row.iter().map(Some)),
        On::Columns(columns) => hash_key(columns.iter().map(|column| column_in(row, *column))),
    }
}

/// Chains the rows of `table` by their key into `order`, whose room is reused: a table that rows
/// can be looked up in by key, with nothing sorted.
///
/// # Errors
///
/// `NoSuchColumn` when `on` names a column the table lacks.
pub(crate) fn index(
    table: &Table,
    on: On<'_>,
    order: &mut KeyIndex,
) -> Result<(), TableModelError> {
    if let On::Columns(columns) = on {
        check_columns(&table.kinds, columns)?;
    }
    let KeyIndex { rows, hashes, .. } = order;
    let count = table.count.0;
    rows.clear();
    rows.extend(positions(table.count));
    hashes.clear();
    hashes.extend(
        rows.iter()
            .map(|each| row_in(table, *each).map_or(KeyCellsHash(0), |row| hash_row(row, on))),
    );
    chain(order, count);
    Ok(())
}

/// Link indexed row hashes in original order, reusing all bucket and chain allocations.
fn chain(order: &mut KeyIndex, count: usize) {
    let buckets = count.next_power_of_two().max(1);
    order.heads.clear();
    order.heads.resize(buckets, ChainLink(0));
    order.next.clear();
    order.next.resize(count, ChainLink(0));
    // Last row first, so a chain is met from the first row to the last.
    for (place, (hash, link)) in order
        .hashes
        .iter()
        .zip(order.next.iter_mut())
        .enumerate()
        .rev()
    {
        let bucket = usize::try_from(hash.0).unwrap_or(0) & (buckets - 1);
        if let Some(head) = order.heads.get_mut(bucket) {
            *link = *head;
            *head = ChainLink(u32::try_from(place + 1).unwrap_or(u32::MAX));
        }
    }
}

/// Whether every declared stored column has exactly the first column's row population.
/// Other views keep the materializing path, including its treatment of incomplete rows.
pub(crate) fn rectangular(view: ColumnsView<'_>) -> bool {
    let count = view.columns.first().map_or(0, |column| column.0.len());
    view.columns.len() == view.kinds.len()
        && view.columns.iter().all(|column| column.0.len() == count)
}

/// Whether the actual relation reader supplies one complete, stable file population for
/// successful-result reuse. The caller must pass the source the executor will actually read;
/// native syntax rows come from the file-owned tree even when the facts adapter exposes a
/// vacant relation view. That tree and its source generation remain unchanged for the rule loop.
pub(crate) fn complete_for_reuse(
    relation: Relation,
    rows: &Rows<'_>,
    partials: &[Partial],
    stable: bool,
) -> bool {
    if !stable || partials.iter().any(|partial| partial.relation == relation) {
        return false;
    }
    match rows {
        Rows::Stored(view) => {
            rectangular(*view)
                && view.kinds == kinds(relation)
                && view
                    .columns
                    .first()
                    .is_none_or(|column| column.0.len() <= u32::MAX as usize)
        }
        Rows::Tree(tree) if relation == Relation::SyntaxTree && tree.form == TreeForm::Elements => {
            let elements = tree.elements;
            let count = real_count(elements).0;
            let names = kind_names(elements);
            syntax_acceptance(elements) == SyntaxAcceptance::Accepted
                && !has_incomplete_syntax(elements)
                && count <= u32::MAX as usize
                && elements.start.len() == count
                && elements.end.len() == count
                && elements.text.len() == count
                && elements
                    .kind
                    .iter()
                    .all(|kind| usize::from(kind.0) < names.len())
        }
        Rows::Tree(_) | Rows::Computed(_) | Rows::Selected(_) => false,
    }
}

/// Decode one typed stored cell, preserving normalization such as nonzero flag encodings.
fn stored_cell(view: ColumnsView<'_>, row: RowIndex, column: ColumnIndex) -> Option<Value> {
    let place = usize::try_from(column.0).ok()?;
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
            kinds: &view.selection.kinds,
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
    let physical = RowIndex(usize::try_from(*view.positions.get(row.0)?).ok()?);
    stored_cell(view.source, physical, *view.columns.get(column.0 as usize)?)
}

/// Read one typed cell without decoding unused stored payload or allocating a row buffer.
pub(crate) fn cell_at(rows: &Rows<'_>, row: RowIndex, column: ColumnIndex) -> Option<Value> {
    crate::domain::table_model::collections::representations::service::borrowed(rows)
        .cell(row, column)
}

/// Index only projected keys of a compact population, preserving logical duplicate row positions.
pub(crate) fn index_selected_view(
    view: super::types::SelectedView<'_>,
    columns: &[ColumnIndex],
    order: &mut KeyIndex,
) -> Result<(), TableModelError> {
    check_columns(view.kinds, columns)?;
    order.rows.clear();
    order.rows.extend(positions(RowCount(view.positions.len())));
    order.hashes.clear();
    order.hashes.extend(order.rows.iter().map(|row| {
        columns.iter().fold(KeyCellsHash(0), |hash, column| {
            mix(
                hash,
                selected_view_cell(view, *row, *column)
                    .map_or(HashWord(0), |value| word_of(&value)),
            )
        })
    }));
    chain(order, view.positions.len());
    Ok(())
}

/// Index only the typed key cells of a rectangular stored relation, retaining original row IDs.
///
/// # Errors
///
/// `NoSuchColumn` when a key names a column the view lacks, also for an empty view.
pub(crate) fn index_stored(
    view: ColumnsView<'_>,
    columns: &[ColumnIndex],
    order: &mut KeyIndex,
) -> Result<(), TableModelError> {
    check_columns(view.kinds, columns)?;
    let count = view.columns.first().map_or(0, |column| column.0.len());
    order.rows.clear();
    order.rows.extend(positions(RowCount(count)));
    order.hashes.clear();
    order.hashes.extend(order.rows.iter().map(|row| {
        columns.iter().fold(KeyCellsHash(0), |hash, column| {
            let word = stored_cell(view, *row, *column).map_or(HashWord(0), |cell| word_of(&cell));
            mix(hash, word)
        })
    }));
    chain(order, count);
    Ok(())
}

/// Find original stored rows by typed keys, checking equality after every hash match.
/// The caller indexes a rectangular view first; payload columns are never read here.
pub(crate) fn matching_stored<'o, 'k>(
    view: ColumnsView<'o>,
    columns: &'k [ColumnIndex],
    order: &'o KeyIndex,
    key: &'k [Value],
) -> impl Iterator<Item = &'o RowIndex> + Clone {
    let hash = hash_key(key.iter().map(Some));
    let bucket = usize::try_from(hash.0).unwrap_or(0) & order.heads.len().saturating_sub(1);
    let mut at = order.heads.get(bucket).copied().unwrap_or(ChainLink(0));
    std::iter::from_fn(move || {
        while at.0 != 0 {
            let place = usize::try_from(at.0 - 1).unwrap_or(usize::MAX);
            at = order.next.get(place).copied().unwrap_or(ChainLink(0));
            if order.hashes.get(place) == Some(&hash)
                && columns.iter().zip(key).all(|(column, wanted)| {
                    stored_cell(view, RowIndex(place), *column) == Some(*wanted)
                })
            {
                return order.rows.get(place);
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

/// Build equality chains from a borrowed column accessor, retaining only row IDs/hashes.
pub(crate) fn index_cells(
    count: RowCount,
    columns: &[ColumnIndex],
    order: &mut KeyIndex,
    cell: impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
) {
    order.rows.clear();
    order.rows.extend(positions(count));
    order.hashes.clear();
    order.hashes.extend(order.rows.iter().map(|row| {
        columns.iter().fold(KeyCellsHash(0), |hash, column| {
            mix(
                hash,
                cell(*row, *column).map_or(HashWord(0), |value| word_of(&value)),
            )
        })
    }));
    chain(order, count.0);
}
