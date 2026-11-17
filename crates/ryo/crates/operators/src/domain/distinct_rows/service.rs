//! A table's rows kept once each through the operators' one distinct kernel, for a caller that holds a whole table rather than
//! a node's output.
use crate::domain::physical_plan::collections::kernels::distinct::item::counted;
use crate::domain::physical_plan::types::CellStart;
use crate::domain::physical_plan::types::row_set::value::RowSet;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::rows::types::{RowCount, Table};

/// Keeps each row of `table` once, the first of its kind, in order, as the distinct kernel keeps a node's rows.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::distinct_rows::service::kept_once;
/// use ryo_values::domain::rows::types::Table;
/// let mut table = Table::default();
/// kept_once(&mut table);
/// assert_eq!(table.count.0, 0);
/// ```
pub fn kept_once(table: &mut Table) {
    let rows = std::mem::take(&mut table.cells);
    let count = std::mem::replace(&mut table.count, RowCount(0));
    let width = table.kinds.len();
    let mut seen = RowSet::default();
    for at in 0..count.0 {
        let start = at.saturating_mul(width);
        let Some(row) = rows.get(start..start.saturating_add(width)) else {
            break;
        };
        let place = CellStart(table.cells.len());
        table.cells.extend_from_slice(row);
        counted(table, place, RepeatedRows::Dropped, &mut seen);
    }
}
