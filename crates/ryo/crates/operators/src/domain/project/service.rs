//! Select columns in plan order, preserving duplicates and the first missing-column error.
use crate::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// Append the selected cells to an engine-owned scratch row.
/// The caller clears the row before this operation and publishes it only on success.
pub fn project(
    row: &[Value],
    columns: &[ColumnIndex],
    projected: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    for column in columns {
        let index = usize::try_from(column.0)
            .map_err(|_| PortableOperatorFault::NoSuchColumn { index: column.0 })?;
        projected.push(
            row.get(index)
                .copied()
                .ok_or(PortableOperatorFault::NoSuchColumn { index: column.0 })?,
        );
    }
    Ok(())
}
