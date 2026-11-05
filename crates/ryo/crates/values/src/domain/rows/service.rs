//! Plain row access, independent of any invocation or population owner.
use super::errors::TableModelError;
use crate::domain::plan_parts::types::ColumnIndex;
use ryo_relations::domain::relations::types::Value;
pub fn column_in<T>(items: &[T], index: ColumnIndex) -> Option<&T> {
    items.get(usize::try_from(index.0).unwrap_or(usize::MAX))
}
pub fn cell(row: &[Value], index: ColumnIndex) -> Result<Value, TableModelError> {
    column_in(row, index)
        .copied()
        .ok_or(TableModelError::NoSuchColumn { index: index.0 })
}

use super::types::{RowCount, RowIndex, Table};
use ryo_relations::domain::relations::types::CellKind;
pub fn row_in(table: &Table, index: RowIndex) -> Option<&[Value]> {
    let (width, index) = (table.kinds.len(), index.0);
    table
        .cells
        .get(index * width..(index + 1) * width)
        .filter(|_| index < table.count.0)
}
pub fn reset(table: &mut Table, kinds: &[CellKind]) {
    table.kinds.clear();
    table.kinds.extend_from_slice(kinds);
    table.cells.clear();
    table.count = RowCount(0);
}
pub fn push_row(table: &mut Table, row: &[Value]) {
    table.cells.extend_from_slice(row);
    table.count = RowCount(table.count.0 + 1);
}

/// The valid row indices in source order, without allocating storage.
pub fn positions(count: RowCount) -> impl Iterator<Item = RowIndex> + Clone {
    (0..count.0).map(RowIndex)
}
