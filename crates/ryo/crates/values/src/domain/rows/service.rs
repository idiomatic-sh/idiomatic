//! Plain row access, independent of any invocation or population owner.
use super::errors::RowsError;
use crate::domain::plan_parts::types::ColumnIndex;
use ryo_relations::domain::relations::types::Value;
/// The item at column `index`, or `None` when the slice is shorter.
pub fn column_in<T>(items: &[T], index: ColumnIndex) -> Option<&T> {
    items.get(usize::try_from(index.0).unwrap_or(usize::MAX))
}

use super::types::{RowCount, RowIndex, Table};
use ryo_relations::domain::relations::types::CellKind;

/// Appends one row, which must be as wide as the table's columns.
pub fn push_row(table: &mut Table, row: &[Value]) {
    table.cells.extend_from_slice(row);
    table.count = RowCount(table.count.0 + 1);
}
