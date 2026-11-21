//! The services of the `stages` concept: one compiled loop for each kind of stage, over the columns of a batch. A loop reads
//! slices and writes vectors; none runs a step program, reads a clock or counts a row: its caller adds a batch's rows to the
//! work once, after the loop.

use super::types::{BatchKey, BatchRow, BatchValue, Batches, CellTest, FanRows, KeptWhen, RowKept};
use crate::domain::indexes::reading::service::{cell_at, rows_of, run_of};
use crate::domain::indexes::types::{ColumnView, IndexRead, SourceRow};
use ryo_language::domain::facts::types::CellWord;
use ryo_operators::domain::ordered_groups::service::nearest;
use ryo_operators::domain::ordered_groups::types::GroupOrders;
use ryo_relations::domain::relations::types::{Count, Integer};
use ryo_values::domain::plan_orders::types::OrderBound;

/// A row's number as the handle a batch holds of it.
#[inline]
pub(crate) fn handle(row: Count) -> CellWord {
    CellWord(u64::try_from(row.0).unwrap_or(u64::MAX))
}

/// The row a handle names.
#[inline]
pub(crate) fn row_of(named: CellWord) -> Count {
    Count(usize::try_from(named.0).unwrap_or(usize::MAX))
}

/// A row of a batch as a selection holds it.
#[inline]
fn selected_row(row: Count) -> BatchRow {
    BatchRow(u32::try_from(row.0).unwrap_or(u32::MAX))
}

/// `Scan` over a whole source: the handles of the rows `first..end`.
pub(crate) fn scan(first: Count, end: Count, out: &mut Vec<CellWord>) {
    out.clear();
    out.extend((first.0..end.0).map(|row| handle(Count(row))));
}

/// `Scan` over one bucket of a source: the handles of an index's rows for one key.
pub(crate) fn scan_bucket(rows: &[SourceRow], out: &mut Vec<CellWord>) {
    out.clear();
    out.extend(rows.iter().map(|row| CellWord(u64::from(row.0))));
}

/// Writes the row's cells of a key into their places.
#[inline]
fn keyed(key: &mut BatchKey<'_>, row: Count) {
    for part in key.parts {
        if let Some(cell) = key.cells.get_mut(part.place.0)
            && let Some(value) = part.column.get(row.0)
        {
            *cell = *value;
        }
    }
}

/// Gives the rows `found` of the row in the cursor is at, as far as the batch has room, and moves the cursor on. Answers
/// whether the batch out is full or the row's rows did not all fit: the loop then stops.
#[inline]
fn fanned(found: impl ExactSizeIterator<Item = CellWord>, fan: &mut FanRows) -> Batches {
    let left = found.len().saturating_sub(fan.cursor.within.0);
    let given = left.min(fan.limit.0.saturating_sub(fan.out.len()));
    fan.out.extend(found.skip(fan.cursor.within.0).take(given));
    fan.up
        .extend(std::iter::repeat_n(selected_row(fan.cursor.row), given));
    if given < left {
        fan.cursor.within = Count(fan.cursor.within.0.saturating_add(given));
        Batches::More
    } else {
        fan.cursor.row = Count(fan.cursor.row.0.saturating_add(1));
        fan.cursor.within = Count(0);
        if fan.out.len() < fan.limit.0 {
            Batches::Done
        } else {
            Batches::More
        }
    }
}

/// Whether a loop that stopped has read every one of `rows` rows in.
#[inline]
fn read_all(fan: &FanRows, rows: Count) -> Batches {
    if fan.cursor.row.0 >= rows.0 {
        Batches::Done
    } else {
        Batches::More
    }
}

/// `Probe` in rows mode: for each of the batch's `rows` rows, from the cursor on, the rows of the index its key holds. Stops
/// when the batch out is full; answers whether every row in was read.
pub(crate) fn probe(
    index: IndexRead<'_>,
    mut key: BatchKey<'_>,
    rows: Count,
    fan: &mut FanRows,
) -> Batches {
    while fan.cursor.row.0 < rows.0 {
        keyed(&mut key, fan.cursor.row);
        let found = rows_of(index, run_of(index, key.cells));
        if fanned(found.iter().map(|row| CellWord(u64::from(row.0))), fan) == Batches::More {
            break;
        }
    }
    read_all(fan, rows)
}

/// `Probe` in present or absent mode: the rows of the batch whose key has a row in the index, or has none, as a selection.
pub(crate) fn probe_keep(
    index: IndexRead<'_>,
    mut key: BatchKey<'_>,
    rows: Count,
    kept: KeptWhen,
    keep: &mut Vec<BatchRow>,
) {
    keep.clear();
    keep.resize(rows.0, BatchRow(0));
    let mut count = 0_usize;
    for row in 0..rows.0 {
        keyed(&mut key, Count(row));
        let run = run_of(index, key.cells);
        if let Some(slot) = keep.get_mut(count) {
            *slot = selected_row(Count(row));
        }
        count = count.saturating_add(usize::from(
            (run.end.0 > run.start.0) == (kept == KeptWhen::Present),
        ));
    }
    keep.truncate(count);
}

/// `Product`: for each of the batch's `rows` rows, from the cursor on, every row of a source of `source` rows. Stops when
/// the batch out is full; answers whether every row in was read.
pub(crate) fn product(rows: Count, source: Count, fan: &mut FanRows) -> Batches {
    while fan.cursor.row.0 < rows.0 {
        if fanned((0..source.0).map(|row| handle(Count(row))), fan) == Batches::More {
            break;
        }
    }
    read_all(fan, rows)
}

/// `Latest`: for each row of the batch, from the cursor on, the rows of its key's group nearest its bound on `side`. The
/// index is of the sorted table by its keys, whose groups are stretches of consecutive rows; `orders` is the table's order
/// column, row by row. The meaning is the operators crate's `nearest`.
pub(crate) fn latest(
    index: IndexRead<'_>,
    orders: &[Integer],
    mut key: BatchKey<'_>,
    bounds: BatchValue<'_>,
    side: OrderBound,
    rows: Count,
    fan: &mut FanRows,
) -> Batches {
    while fan.cursor.row.0 < rows.0 {
        keyed(&mut key, fan.cursor.row);
        let found = rows_of(index, run_of(index, key.cells));
        let first = found
            .first()
            .map_or(0, |row| usize::try_from(row.0).unwrap_or(usize::MAX));
        let group = orders
            .get(first..first.saturating_add(found.len()))
            .unwrap_or_default();
        let bound = Integer(value_at(bounds, fan.cursor.row).0.cast_signed());
        let near = nearest(GroupOrders(group), bound, side).0;
        if fanned(near.map(|at| handle(Count(first.saturating_add(at)))), fan) == Batches::More {
            break;
        }
    }
    read_all(fan, rows)
}

/// The cell a loop reads for `row`.
#[inline]
pub(crate) fn value_at(value: BatchValue<'_>, row: Count) -> CellWord {
    match value {
        BatchValue::Column(column) => match column.get(row.0) {
            Some(cell) => *cell,
            None => CellWord(0),
        },
        BatchValue::Constant(cell) => cell,
    }
}

/// A column of the batch out of a loop that gave several rows for one: for each row out, the cell of the row in it came
/// from.
pub(crate) fn gather(column: &[CellWord], up: &[BatchRow], out: &mut Vec<CellWord>) {
    out.clear();
    out.extend(up.iter().map(
        |row| match column.get(usize::try_from(row.0).unwrap_or(usize::MAX)) {
            Some(cell) => *cell,
            None => CellWord(0),
        },
    ));
}

/// A column of the batch after a loop that kept some rows: the cells of the rows kept, moved to the front in place.
pub(crate) fn compact(column: &mut Vec<CellWord>, keep: &[BatchRow]) {
    for (to, from) in keep.iter().enumerate() {
        let moved = match column.get(usize::try_from(from.0).unwrap_or(usize::MAX)) {
            Some(cell) => *cell,
            None => CellWord(0),
        };
        if let Some(slot) = column.get_mut(to) {
            *slot = moved;
        }
    }
    column.truncate(keep.len());
}

/// `Read`: for each handle of the batch, the cell of that row in a column of the source.
pub(crate) fn read(view: ColumnView<'_>, handles: &[CellWord], out: &mut Vec<CellWord>) {
    out.clear();
    out.extend(handles.iter().map(|each| cell_at(view, row_of(*each))));
}

/// The rows for which `tests` says yes, as a selection: one store and one add per row, no branch.
#[inline]
fn selected(rows: Count, keep: &mut Vec<BatchRow>, tests: impl Iterator<Item = RowKept>) {
    keep.clear();
    keep.resize(rows.0, BatchRow(0));
    let mut count = 0_usize;
    for (row, kept) in tests.take(rows.0).enumerate() {
        if let Some(slot) = keep.get_mut(count) {
            *slot = selected_row(Count(row));
        }
        count = count.saturating_add(usize::from(kept == RowKept::Yes));
    }
    keep.truncate(count);
}

/// The rows for which one comparison holds between two values, each a column or one cell.
#[inline]
fn compared(
    left: BatchValue<'_>,
    right: BatchValue<'_>,
    rows: Count,
    keep: &mut Vec<BatchRow>,
    test: impl Fn(CellWord, CellWord) -> RowKept + Copy,
) {
    match (left, right) {
        (BatchValue::Column(lefts), BatchValue::Column(rights)) => selected(
            rows,
            keep,
            lefts
                .iter()
                .zip(rights)
                .map(|(one, other)| test(*one, *other)),
        ),
        (BatchValue::Column(lefts), BatchValue::Constant(other)) => {
            selected(rows, keep, lefts.iter().map(|one| test(*one, other)));
        }
        (BatchValue::Constant(one), BatchValue::Column(rights)) => {
            selected(rows, keep, rights.iter().map(|other| test(one, *other)));
        }
        (BatchValue::Constant(one), BatchValue::Constant(other)) => {
            selected(rows, keep, std::iter::repeat_n(test(one, other), rows.0));
        }
    }
}

/// What a test that came out true or false says of a row.
macro_rules! holds {
    ($test:expr) => {
        if $test { RowKept::Yes } else { RowKept::No }
    };
}

/// `Keep` of a comparison of two cells: the rows of the batch it holds for, as a selection. An order is of whole numbers.
pub(crate) fn keep(
    test: CellTest,
    left: BatchValue<'_>,
    right: BatchValue<'_>,
    rows: Count,
    keep: &mut Vec<BatchRow>,
) {
    match test {
        CellTest::Equal => compared(left, right, rows, keep, |one, other| holds!(one == other)),
        CellTest::NotEqual => compared(left, right, rows, keep, |one, other| holds!(one != other)),
        CellTest::Less => compared(left, right, rows, keep, |one, other| {
            holds!(one.0.cast_signed() < other.0.cast_signed())
        }),
        CellTest::LessOrEqual => compared(left, right, rows, keep, |one, other| {
            holds!(one.0.cast_signed() <= other.0.cast_signed())
        }),
        CellTest::Greater => compared(left, right, rows, keep, |one, other| {
            holds!(one.0.cast_signed() > other.0.cast_signed())
        }),
        CellTest::GreaterOrEqual => compared(left, right, rows, keep, |one, other| {
            holds!(one.0.cast_signed() >= other.0.cast_signed())
        }),
    }
}

/// `Keep` of a membership: the rows whose cell is in a sorted list of cells, or with `kept` absent the rows whose cell is
/// not.
pub(crate) fn keep_in(
    column: BatchValue<'_>,
    sorted: &[CellWord],
    kept: KeptWhen,
    rows: Count,
    keep: &mut Vec<BatchRow>,
) {
    let test =
        |cell: CellWord| holds!(sorted.binary_search(&cell).is_ok() == (kept == KeptWhen::Present));
    match column {
        BatchValue::Column(cells) => selected(rows, keep, cells.iter().map(|cell| test(*cell))),
        BatchValue::Constant(cell) => {
            selected(rows, keep, std::iter::repeat_n(test(cell), rows.0));
        }
    }
}

/// `Keep` of a test only its caller can decide (a text's): the rows for which `test` says yes, as a selection.
pub(crate) fn keep_by(rows: Count, keep: &mut Vec<BatchRow>, test: impl FnMut(Count) -> RowKept) {
    selected(rows, keep, (0..rows.0).map(Count).map(test));
}
