//! The `distinct` item of the `kernels` collection: a row kept the first time only, its repeats found in the row set of the
//! rows kept, so no row is copied out.

use crate::domain::physical_plan::types::CellStart;
use crate::domain::physical_plan::types::row_set::value::{FirstSeen, RowSet};
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::rows::types::{RowCount, Table};

/// Counts the row `output` holds from cell `start` as one of its rows, unless `repeated` drops it as a repeat of a row it
/// holds, when it is cut off. `repeats` keeps the rows of `output` kept so far, once each.
pub(crate) fn counted(
    output: &mut Table,
    start: CellStart,
    repeated: RepeatedRows,
    repeats: &mut RowSet,
) {
    if repeated == RepeatedRows::Dropped && repeats.first_seen(output, start) == FirstSeen::Repeat {
        output.cells.truncate(start.0);
        return;
    }
    output.count = RowCount(output.count.0 + 1);
}
