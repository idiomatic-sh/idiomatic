//! The services of the `using` part: a `Use` stage after its patterns were asked, once for each distinct key of the batch.
//! The rows a key's pattern yielded are a stretch of the answers; whether it has a row is a mark on each row of the batch.

use crate::domain::indexes::types::RowRun;
use crate::domain::stages::service::{fanned, handle, read_all, selected};
use crate::domain::stages::types::{BatchRow, Batches, FanRows, KeptWhen, RowKept};
use ryo_relations::domain::relations::types::Count;

/// `Use` in rows mode: for each row of the batch, from the cursor on, the rows of the answers its key holds, each by its
/// number among the answers. Stops when the batch out is full; answers whether every row in was read.
pub(crate) fn spread(spans: &[RowRun], rows: Count, fan: &mut FanRows) -> Batches {
    while fan.cursor.row.0 < rows.0 {
        let span = match spans.get(fan.cursor.row.0) {
            Some(span) => *span,
            None => RowRun::default(),
        };
        if fanned(
            (span.start.0..span.end.0).map(|row| handle(Count(row))),
            fan,
        ) == Batches::More
        {
            break;
        }
    }
    read_all(fan, rows)
}

/// `Use` in present or absent mode: the rows of the batch whose key's pattern has a row, or has none, as a selection.
pub(crate) fn keep_marked(
    found: &[RowKept],
    kept: KeptWhen,
    rows: Count,
    keep: &mut Vec<BatchRow>,
) {
    selected(
        rows,
        keep,
        found.iter().map(|each| {
            if (*each == RowKept::Yes) == (kept == KeptWhen::Present) {
                RowKept::Yes
            } else {
                RowKept::No
            }
        }),
    );
}
