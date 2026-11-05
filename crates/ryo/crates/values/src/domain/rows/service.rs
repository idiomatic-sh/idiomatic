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
