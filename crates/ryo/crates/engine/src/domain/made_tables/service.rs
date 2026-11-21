//! The services of the `made_tables` concept: a table's rows written from a batch's columns, with or without the rows it
//! already holds, and the fills that group their input: a counted grouping and a table sorted within each key. A grouping is
//! one index over the gathered input, every key column as one key, and one walk of its keys; the group's row is the
//! operators crate's meaning.

use super::constants::HASH_FOLD;
use super::types::{GroupRoom, TableRows};
use crate::domain::code::types::IndexSite;
use crate::domain::indexes::reading::service::{cell_at, hashed, read, rows_of, runs};
use crate::domain::indexes::service::{begin, clear_indexes, finish, key_column};
use crate::domain::indexes::types::{ColumnView, FileStamp, RowHash, SourceRow};
use crate::domain::stages::types::BatchValue;
use ryo_language::domain::facts::types::CellWord;
use ryo_operators::domain::aggregate::service::group_cells;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::types::{Count, Integer};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// Empties a table for a file, as a table of `width` columns, keeping its room.
pub(crate) fn clear_rows(table: &mut TableRows, width: Count) {
    table.cells.clear();
    table.seen.clear();
    table.orders.clear();
    table.count = Count(0);
    table.width = width;
}

/// One column of a table, as a loop reads it.
#[inline]
pub(crate) fn column(table: &TableRows, column: Count) -> ColumnView<'_> {
    ColumnView {
        cells: &table.cells,
        first: column,
        stride: table.width,
    }
}

/// The cells of one row of a table; none past its end.
#[inline]
pub(crate) fn row(table: &TableRows, row: Count) -> &[CellWord] {
    let from = row.0.saturating_mul(table.width.0);
    table
        .cells
        .get(from..from.saturating_add(table.width.0))
        .unwrap_or_default()
}

/// The hash a table's rows are kept by: every cell mixed in, then the high half folded onto the low, which the map reads.
#[inline]
fn row_hash(cells: &[CellWord]) -> RowHash {
    let hash = hashed(cells);
    RowHash(hash.0 ^ (hash.0 >> HASH_FOLD.0))
}

/// `Write`: appends `rows` rows to a table, the cells of each read from `sources`, one for each column. One pass per
/// column.
pub(crate) fn write(table: &mut TableRows, sources: &[BatchValue<'_>], rows: Count) {
    let base = table.cells.len();
    let width = sources.len();
    table.cells.resize(
        base.saturating_add(rows.0.saturating_mul(width)),
        CellWord(0),
    );
    for (at, source) in sources.iter().enumerate() {
        let slots = table
            .cells
            .get_mut(base.saturating_add(at)..)
            .unwrap_or_default()
            .iter_mut()
            .step_by(width.max(1));
        match *source {
            BatchValue::Column(cells) => {
                for (slot, cell) in slots.zip(cells.iter().take(rows.0)) {
                    *slot = *cell;
                }
            }
            BatchValue::Constant(cell) => {
                for slot in slots.take(rows.0) {
                    *slot = cell;
                }
            }
        }
    }
    table.count = Count(table.count.0.saturating_add(rows.0));
}

/// The cells of the row a table's map holds by number.
#[inline]
fn held(cells: &[CellWord], width: Count, at: SourceRow) -> &[CellWord] {
    let from = usize::try_from(at.0)
        .unwrap_or(usize::MAX)
        .saturating_mul(width.0);
    cells
        .get(from..from.saturating_add(width.0))
        .unwrap_or_default()
}

/// `Write` with repeats dropped: appends each of `rows` rows the table does not hold yet. A row is written at the table's
/// end, looked up by what it holds, and taken back when the table held it.
pub(crate) fn write_new(table: &mut TableRows, sources: &[BatchValue<'_>], rows: Count) {
    let TableRows {
        cells, seen, count, ..
    } = table;
    let width = Count(sources.len());
    for at in 0..rows.0 {
        let base = cells.len();
        cells.extend(sources.iter().map(|source| match *source {
            BatchValue::Column(column) => match column.get(at) {
                Some(cell) => *cell,
                None => CellWord(0),
            },
            BatchValue::Constant(cell) => cell,
        }));
        let (before, written) = cells.split_at(base);
        let hash = row_hash(written);
        let repeated = seen
            .find(hash.0, |other| held(before, width, *other) == written)
            .is_some();
        if repeated {
            cells.truncate(base);
        } else {
            let number = SourceRow(u32::try_from(count.0).unwrap_or(u32::MAX));
            drop(seen.insert_unique(hash.0, number, |other| {
                row_hash(held(cells, width, *other)).0
            }));
            *count = Count(count.0.saturating_add(1));
        }
    }
}

/// Builds the index of a gathered input by its key columns, sorted within a key by `order` when one is given, as the one
/// index of the room.
fn grouped(input: &TableRows, room: &mut GroupRoom, order: Option<Count>) {
    clear_indexes(&mut room.index);
    begin(&mut room.index, input.count, Count(room.keys.len()));
    for (at, key) in room.keys.iter().enumerate() {
        let place = Count(usize::try_from(key.0).unwrap_or(usize::MAX));
        key_column(&mut room.index, Count(at), column(input, place));
    }
    finish(
        &mut room.index,
        IndexSite(0),
        FileStamp(0),
        order.map(|place| column(input, place)),
    );
}

/// The `Group(Count)` fill: one row out for each key of the gathered `input`, which is the key's cells and then how many
/// rows hold it, or with `distinct` how many different cells of that column they hold. The keys are `room.keys`. Linear in
/// the input: one index, one walk of its keys.
///
/// # Errors
///
/// `NoSuchColumn` when a key names no column of the input: a fault of the encoding, never of a file.
pub(crate) fn count_groups(
    input: &TableRows,
    distinct: Option<Count>,
    room: &mut GroupRoom,
    out: &mut TableRows,
) -> Result<(), PortableOperatorFault> {
    grouped(input, room, None);
    let GroupRoom {
        index,
        keys,
        row: written,
        values,
    } = room;
    let index = read(index, IndexSite(0));
    for run in runs(index) {
        let rows = rows_of(index, run);
        let Some(first) = rows.first() else {
            continue;
        };
        let count = match distinct {
            None => rows.len(),
            Some(place) => {
                values.clear();
                values.extend(rows.iter().map(|each| {
                    cell_at(
                        column(input, place),
                        Count(usize::try_from(each.0).unwrap_or(usize::MAX)),
                    )
                }));
                values.sort_unstable();
                values.dedup();
                values.len()
            }
        };
        written.clear();
        group_cells(
            row(input, Count(usize::try_from(first.0).unwrap_or(usize::MAX))),
            keys,
            CellWord(u64::try_from(count).unwrap_or(u64::MAX)),
            None,
            written,
        )?;
        out.cells.extend_from_slice(written);
        out.count = Count(out.count.0.saturating_add(1));
    }
    Ok(())
}

/// The `Ordered(Sorted)` fill: the rows of the gathered `input`, those of one key together and ascending by the column
/// `order`, with that column kept beside the rows as the numbers a `Latest` searches. The keys are `room.keys`. One index and
/// a sort of each key's rows.
pub(crate) fn sort_groups(
    input: &TableRows,
    order: Count,
    room: &mut GroupRoom,
    out: &mut TableRows,
) {
    grouped(input, room, Some(order));
    let index = read(&room.index, IndexSite(0));
    let orders = column(input, order);
    for run in runs(index) {
        for each in rows_of(index, run) {
            let place = Count(usize::try_from(each.0).unwrap_or(usize::MAX));
            out.cells.extend_from_slice(row(input, place));
            out.orders
                .push(Integer(cell_at(orders, place).0.cast_signed()));
        }
        out.count = Count(
            out.count
                .0
                .saturating_add(run.end.0.saturating_sub(run.start.0)),
        );
    }
}

/// The key columns of a fill: written into the room before a grouping fill.
pub(crate) fn set_keys(room: &mut GroupRoom, keys: impl Iterator<Item = ColumnIndex>) {
    room.keys.clear();
    room.keys.extend(keys);
}
