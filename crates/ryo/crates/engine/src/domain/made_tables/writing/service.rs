//! The services of the `writing` part: rows appended to a table from a batch's columns, with or without the rows it
//! already holds, into the table's own list or at the end of the store's arena of the file's tables; and a table's
//! cells kept the last of the arena while its fill runs.

use crate::domain::indexes::reading::service::{folded, hashed};
use crate::domain::indexes::types::{RowHash, SourceRow};
use crate::domain::made_tables::columns::types::CellsLie;
use crate::domain::made_tables::types::{FillStore, TableRows};
use crate::domain::stages::types::BatchValue;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// The hash a table's rows are kept by: every cell mixed in, then the high half folded onto the low, which the map reads.
#[inline]
fn row_hash(cells: &[CellWord]) -> RowHash {
    folded(hashed(cells))
}

/// `Write`: appends `rows` rows of a table at the end of `cells`, where the table's cells lie (its own list, or the
/// store's arena, the table's being the last cells of it), the cells of each read from `sources`, one for each column
/// the table keeps. One pass per column.
pub(crate) fn written(
    cells: &mut Vec<CellWord>,
    table: &mut TableRows,
    sources: &[BatchValue<'_>],
    rows: Count,
) {
    let base = cells.len();
    let width = sources.len();
    cells.resize(
        base.saturating_add(rows.0.saturating_mul(width)),
        CellWord(0),
    );
    for (at, source) in sources.iter().enumerate() {
        let slots = cells
            .get_mut(base.saturating_add(at)..)
            .unwrap_or_default()
            .iter_mut()
            .step_by(width.max(1));
        match *source {
            BatchValue::Column(column) => {
                for (slot, cell) in slots.zip(column.iter().take(rows.0)) {
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

/// `Write` with repeats dropped: appends each of `rows` rows the table does not hold yet. The table's cells lie in
/// `cells` from `start` to its end. A row is written at the end, looked up by what it holds, and taken back when the
/// table held it.
pub(crate) fn written_new(
    cells: &mut Vec<CellWord>,
    start: Count,
    table: &mut TableRows,
    sources: &[BatchValue<'_>],
    rows: Count,
) {
    let TableRows { seen, count, .. } = table;
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
        let own = before.get(start.0..).unwrap_or_default();
        let hash = row_hash(written);
        let repeated = seen
            .find(hash.0, |other| held(own, width, *other) == written)
            .is_some();
        if repeated {
            cells.truncate(base);
        } else {
            let number = SourceRow(u32::try_from(count.0).unwrap_or(u32::MAX));
            let own = cells.get(start.0..).unwrap_or_default();
            drop(seen.insert_unique(hash.0, number, |other| row_hash(held(own, width, *other)).0));
            *count = Count(count.0.saturating_add(1));
        }
    }
}

/// Makes a table's cells the last of the arena before it writes more of them: when another table was written after
/// them (a table made while this one's fill ran), they are moved to the arena's end, and the cells they leave are not
/// used again in the file. Answers where the table's cells start.
pub(crate) fn settled(arena: &mut Vec<CellWord>, table: &mut TableRows) -> Count {
    let start = match table.lies {
        CellsLie::Arena(start) => start,
        CellsLie::Own => Count(arena.len()),
    };
    let held = table.count.0.saturating_mul(table.width.0);
    if start.0.saturating_add(held) == arena.len() {
        return start;
    }
    let moved = Count(arena.len());
    arena.extend_from_within(start.0.min(moved.0)..start.0.saturating_add(held).min(moved.0));
    table.lies = CellsLie::Arena(moved);
    moved
}

/// Puts into the arena the cells a fill wrote in the table's own list, and answers the list emptied: the table is then
/// a stretch of the arena like every other.
fn moved_in(arena: &mut Vec<CellWord>, table: &mut TableRows) -> Vec<CellWord> {
    let mut own = std::mem::take(&mut table.cells);
    if table.lies == CellsLie::Own {
        table.lies = CellsLie::Arena(Count(arena.len()));
        arena.extend_from_slice(&own);
    }
    own.clear();
    own
}

/// Lends a table what its fill writes with: the set of the rows it holds, which only a fill that drops repeats
/// reads; and, when the fill does not write rows straight at the arena's end (`lies` says which), a list of its own.
pub(crate) fn fill_begun(store: &mut FillStore, table: &mut TableRows, lies: CellsLie) {
    table.seen = store.seens.pop().unwrap_or_default();
    table.lies = lies;
    if lies == CellsLie::Own {
        table.cells = store.own_cells.pop().unwrap_or_default();
    }
}

/// Takes back what `fill_begun` lent, the moment the fill ends: the table's cells are in the arena, its own list and
/// its set are emptied and kept by the store for the next fill. After it a table keeps nothing of its own but its slot.
pub(crate) fn fill_ended(store: &mut FillStore, table: &mut TableRows) {
    let own = moved_in(&mut store.table_cells, table);
    if own.capacity() != 0 {
        store.own_cells.push(own);
    }
    set_emptied(&store.table_cells, table);
    store.seens.push(std::mem::take(&mut table.seen));
}

/// How many times larger than its table a set of rows held must be for the table's rows to be taken out one by one.
const SPARSE_SET: Count = Count(16);

/// Empties a table's set of the rows it holds. A set is emptied whole by writing every one of its control bytes, as
/// many as the largest table that ever used it has rows, and most tables are a handful of rows in a set that large:
/// a table far smaller than its set takes its own rows out instead, each found by what it holds.
fn set_emptied(cells: &[CellWord], table: &mut TableRows) {
    let TableRows {
        seen,
        count,
        width,
        lies,
        ..
    } = table;
    if seen.is_empty() {
        return;
    }
    let CellsLie::Arena(start) = *lies else {
        seen.clear();
        return;
    };
    if seen.len().saturating_mul(SPARSE_SET.0) >= seen.capacity() {
        seen.clear();
        return;
    }
    let own = cells.get(start.0..).unwrap_or_default();
    for number in 0..count.0 {
        let row = SourceRow(u32::try_from(number).unwrap_or(u32::MAX));
        if let Ok(entry) =
            seen.find_entry(row_hash(held(own, *width, row)).0, |other| *other == row)
        {
            let _taken = entry.remove();
        }
    }
    // A row that was not found leaves the set to be emptied whole.
    if !seen.is_empty() {
        seen.clear();
    }
}
