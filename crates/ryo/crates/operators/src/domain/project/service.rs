//! Select columns in plan order, preserving duplicates and the first missing-column error.
use crate::domain::contract::types::ProjectColumnAbsent;
use ryo_language::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// Append the selected cells to an engine-owned scratch row.
/// The caller clears the row before this operation and publishes it only on success.
pub fn project(
    row: &[Value],
    columns: &[ColumnIndex],
    projected: &mut Vec<Value>,
) -> Result<(), ProjectColumnAbsent> {
    for column in columns {
        let index = usize::try_from(column.0).map_err(|_| ProjectColumnAbsent(column.0))?;
        projected.push(
            row.get(index)
                .copied()
                .ok_or(ProjectColumnAbsent(column.0))?,
        );
    }
    Ok(())
}
