//! The services of the `made_tables` concept: a table's rows written from a batch's columns, with or without the rows it
//! already holds, and the fills that group their input: a counted grouping and a table sorted within each key. A grouping is
//! one index over the gathered input, every key column as one key, and one walk of its keys; the group's row is the
//! operators crate's meaning.

use super::columns::types::{CellsLie, ColumnIs, FileColumn, RowColumn};
use super::types::{GroupRoom, TableRows};
use super::writing::service::{written, written_new};
use crate::domain::code::types::IndexSite;
use crate::domain::indexes::reading::service::{cell_at, read, rows_of, runs};
use crate::domain::indexes::service::{begin, clear_indexes, finish, fixed_keys, key_column};
use crate::domain::indexes::types::{
    ColumnView, FileStamp, FixedKeys, IndexArena, KeyMask, SourceRow,
};
use crate::domain::stages::types::BatchValue;
use ryo_language::domain::facts::types::CellWord;
use ryo_operators::domain::aggregate::service::group_cells;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::domain::ordered_groups::service::extreme;
use ryo_operators::domain::ordered_groups::types::GroupOrders;
use ryo_relations::domain::relations::types::{Count, Integer};
use ryo_values::domain::plan_orders::types::GroupEnd;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// Empties a table for a file, as a table of `width` columns each kept in its rows, keeping its room.
pub(crate) fn clear_rows(table: &mut TableRows, width: Count) {
    table.cells.clear();
    table.seen.clear();
    table.orders.clear();
    table.count = Count(0);
    table.width = width;
    table.lies = CellsLie::Own;
    table.places.clear();
    table
        .places
        .extend((0..width.0).map(|at| RowColumn::Kept(Count(at))));
}

/// Says which columns of an emptied table are the file's own: no cell is kept for them, the other columns close up, and
/// a reader of one is handed `file`. The one place that decides how a table's columns lie.
pub(crate) fn file_columns(
    table: &mut TableRows,
    file: CellWord,
    is_file: impl Fn(Count) -> ColumnIs,
) {
    let mut kept = 0_usize;
    for (at, place) in table.places.iter_mut().enumerate() {
        *place = match is_file(Count(at)) {
            ColumnIs::TheFile => RowColumn::File,
            ColumnIs::Written => {
                kept = kept.saturating_add(1);
                RowColumn::Kept(Count(kept.saturating_sub(1)))
            }
        };
    }
    table.width = Count(kept);
    table.file = FileColumn([file]);
}

/// Lays an emptied table's columns as another's: what a fill that hands on its input's rows whole does.
pub(crate) fn columns_as(table: &mut TableRows, other: &TableRows) {
    table.places.clear();
    table.places.extend_from_slice(&other.places);
    table.width = other.width;
    table.file = other.file;
}

/// Where a column of a table lies; at its own number when the table was not told.
#[inline]
pub(crate) fn place_of(table: &TableRows, column: Count) -> RowColumn {
    match table.places.get(column.0) {
        Some(place) => *place,
        None => RowColumn::Kept(column),
    }
}

/// One column of a table that keeps its own list (a fill's gathered input; a made table's cells lie in the store's
/// arena and are read through `reading::service`): the file's own is the file's cell for every row.
#[inline]
pub(crate) fn column(table: &TableRows, column: Count) -> ColumnView<'_> {
    match place_of(table, column) {
        RowColumn::Kept(first) => ColumnView {
            cells: &table.cells,
            first,
            stride: table.width,
        },
        RowColumn::File => ColumnView {
            cells: &table.file.0,
            first: Count(0),
            stride: Count(0),
        },
    }
}

/// The cells a table keeps of one row; none past its end.
#[inline]
pub(crate) fn row(table: &TableRows, row: Count) -> &[CellWord] {
    let from = row.0.saturating_mul(table.width.0);
    table
        .cells
        .get(from..from.saturating_add(table.width.0))
        .unwrap_or_default()
}

/// `Write` into a table whose cells lie in its own list.
pub(crate) fn write(table: &mut TableRows, sources: &[BatchValue<'_>], rows: Count) {
    let mut cells = std::mem::take(&mut table.cells);
    written(&mut cells, table, sources, rows);
    table.cells = cells;
}

/// `Write` with repeats dropped into a table whose cells lie in its own list.
pub(crate) fn write_new(table: &mut TableRows, sources: &[BatchValue<'_>], rows: Count) {
    let mut cells = std::mem::take(&mut table.cells);
    written_new(&mut cells, Count(0), table, sources, rows);
    table.cells = cells;
}

/// Builds the index of a gathered input by its key columns, sorted within a key by `order` when one is given, as the one
/// index of the room.
pub(crate) fn grouped(input: &TableRows, room: &mut GroupRoom, order: Option<Count>) {
    let keys = room
        .keys
        .iter()
        .map(|key| Count(usize::try_from(key.0).unwrap_or(usize::MAX)));
    rows_indexed(input, keys, order, &mut room.index);
}

/// Which of a key's columns are the file's own in a table: the bits an index is told, so that it keeps no cell of them.
pub(crate) fn key_mask(table: &TableRows, keys: impl Iterator<Item = Count>) -> KeyMask {
    let mut mask = 0_u32;
    for (at, key) in keys.enumerate() {
        if place_of(table, key) == RowColumn::File {
            mask |= 1_u32
                .checked_shl(u32::try_from(at).unwrap_or(u32::MAX))
                .unwrap_or(0);
        }
    }
    KeyMask(mask)
}

/// The file's cell a table hands for its file columns.
#[inline]
pub(crate) fn file_of(table: &TableRows) -> CellWord {
    let [cell] = table.file.0;
    cell
}

/// Builds, as the one index of `arena`, the index of a table's rows by the key columns given, sorted within a key by
/// `order` when one is given: what a fill that groups its input walks.
pub(crate) fn rows_indexed(
    input: &TableRows,
    keys: impl ExactSizeIterator<Item = Count> + Clone,
    order: Option<Count>,
    arena: &mut IndexArena,
) {
    clear_indexes(arena);
    // A key column that is the file's tells no two rows apart: the key is the other columns.
    let kept = keys
        .clone()
        .filter(|key| place_of(input, *key) != RowColumn::File);
    begin(arena, input.count, Count(kept.clone().count()));
    for (at, key) in kept.enumerate() {
        key_column(arena, Count(at), column(input, key));
    }
    fixed_keys(
        arena,
        FixedKeys {
            skipped: key_mask(input, keys),
            cell: file_of(input),
        },
    );
    finish(
        arena,
        IndexSite(0),
        FileStamp(0),
        order.map(|place| column(input, place)),
    );
}

/// The `Group(Ends)` fill: the rows of the gathered `input` that stand at one end of their key's rows by the column
/// `order`: every row tied for the least, or for the greatest. The keys are `room.keys`. One ordered index, and for each
/// key the operators crate's `extreme` over its orders.
pub(crate) fn end_groups(
    input: &TableRows,
    order: Count,
    end: GroupEnd,
    room: &mut GroupRoom,
    out: &mut TableRows,
) {
    grouped(input, room, Some(order));
    columns_as(out, input);
    let GroupRoom { index, orders, .. } = room;
    let index = read(index, IndexSite(0));
    let ordered = column(input, order);
    for run in runs(index) {
        let rows = rows_of(index, run);
        orders.clear();
        orders.extend(rows.iter().map(|each| {
            Integer(
                cell_at(
                    ordered,
                    Count(usize::try_from(each.0).unwrap_or(usize::MAX)),
                )
                .0
                .cast_signed(),
            )
        }));
        let ended: &[SourceRow] = match rows.get(extreme(GroupOrders(orders), end).0) {
            Some(ended) => ended,
            None => &[],
        };
        for each in ended {
            out.cells.extend_from_slice(row(
                input,
                Count(usize::try_from(each.0).unwrap_or(usize::MAX)),
            ));
            out.count = Count(out.count.0.saturating_add(1));
        }
    }
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
        held,
        ..
    } = room;
    // The table's row is the keys, then the count: a key that is the file's own column is the file's in the table too,
    // and the group's row is made of the cells the input keeps.
    held.clear();
    for key in keys.iter() {
        if let RowColumn::Kept(place) =
            place_of(input, Count(usize::try_from(key.0).unwrap_or(usize::MAX)))
        {
            held.push(ColumnIndex(u32::try_from(place.0).unwrap_or(u32::MAX)));
        }
    }
    file_columns(out, file_of(input), |at| match keys.get(at.0) {
        Some(key) => match place_of(input, Count(usize::try_from(key.0).unwrap_or(usize::MAX))) {
            RowColumn::File => ColumnIs::TheFile,
            RowColumn::Kept(_) => ColumnIs::Written,
        },
        None => ColumnIs::Written,
    });
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
            held,
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
    columns_as(out, input);
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
