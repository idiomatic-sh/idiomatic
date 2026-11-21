//! The services of the `ranging` part: `Probe` with a range. The index is ordered: each key's rows lie sorted by the order
//! column, so the rows between two bounds are one stretch found by two searches.

use crate::domain::indexes::reading::service::{rows_of, run_between, run_of};
use crate::domain::indexes::types::{IndexRead, RangeEnd, RowRun};
use crate::domain::stages::service::{fanned, keyed, read_all, selected_row, value_at};
use crate::domain::stages::types::{BatchKey, BatchRow, Batches, FanRows, KeptWhen, KeyRange};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// The stretch of a key's rows whose order cells lie between the row's two bounds.
#[inline]
fn between(index: IndexRead<'_>, key: &[CellWord], range: KeyRange<'_>, row: Count) -> RowRun {
    run_between(
        index,
        run_of(index, key),
        range.order,
        Some(RangeEnd {
            cell: value_at(range.low, row),
            end: range.low_end,
        }),
        Some(RangeEnd {
            cell: value_at(range.high, row),
            end: range.high_end,
        }),
    )
}

/// `Probe` in rows mode with a range: for each row of the batch, from the cursor on, the rows of its key whose order cell
/// lies between its two bounds. Stops when the batch out is full; answers whether every row in was read.
pub(crate) fn probe_between(
    index: IndexRead<'_>,
    mut key: BatchKey<'_>,
    range: KeyRange<'_>,
    rows: Count,
    fan: &mut FanRows,
) -> Batches {
    while fan.cursor.row.0 < rows.0 {
        keyed(&mut key, fan.cursor.row);
        let found = rows_of(index, between(index, key.cells, range, fan.cursor.row));
        if fanned(found.iter().map(|row| CellWord(u64::from(row.0))), fan) == Batches::More {
            break;
        }
    }
    read_all(fan, rows)
}

/// `Probe` in present or absent mode with a range: the rows of the batch whose key has a row between their two bounds, or
/// has none, as a selection.
pub(crate) fn probe_keep_between(
    index: IndexRead<'_>,
    mut key: BatchKey<'_>,
    range: KeyRange<'_>,
    rows: Count,
    kept: KeptWhen,
    keep: &mut Vec<BatchRow>,
) {
    keep.clear();
    keep.resize(rows.0, BatchRow(0));
    let mut count = 0_usize;
    for row in 0..rows.0 {
        keyed(&mut key, Count(row));
        let run = between(index, key.cells, range, Count(row));
        if let Some(slot) = keep.get_mut(count) {
            *slot = selected_row(Count(row));
        }
        count = count.saturating_add(usize::from(
            (run.end.0 > run.start.0) == (kept == KeptWhen::Present),
        ));
    }
    keep.truncate(count);
}
