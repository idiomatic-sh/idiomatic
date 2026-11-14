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
            .map_err(|_| PortableOperatorFault::NoSuchColumn { index: *column })?;
        projected.push(
            row.get(index)
                .copied()
                .ok_or(PortableOperatorFault::NoSuchColumn { index: *column })?,
        );
    }
    Ok(())
}

/// Projects a scheduled row onto its columns; a projection that drops repeated rows emits each projected
/// row the first time the invocation sees it.
pub(crate) fn evaluate(
    row: &[Value],
    columns: &[ColumnIndex],
    distinct: ryo_values::domain::plan_parts::types::RepeatedRows,
    driver: &mut dyn crate::domain::contract::types::RowDriver,
    projected: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    projected.clear();
    project(row, columns, projected)?;
    if distinct == ryo_values::domain::plan_parts::types::RepeatedRows::Dropped
        && driver.first_time(projected)? != crate::domain::contract::types::FirstOccurrence::First
    {
        return Ok(());
    }
    driver.emit(projected)
}

/// The one lowering of a projection: its columns and repeated-row choice, for the `project` kernel.
pub(crate) fn lower(
    columns: &[ColumnIndex],
    distinct: ryo_values::domain::plan_parts::types::RepeatedRows,
) -> super::types::ProjectParts {
    super::types::ProjectParts {
        columns: columns.to_vec(),
        distinct,
    }
}
