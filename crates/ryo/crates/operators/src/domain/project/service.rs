//! Select columns in plan order, preserving duplicates and the first missing-column error.
use crate::domain::contract::types::PortableOperatorFault;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// Append the selected cells to an engine-owned scratch row.
/// The caller clears the row before this operation and publishes it only on success.
pub(crate) fn project<C: Copy>(
    row: &[C],
    columns: &[ColumnIndex],
    projected: &mut Vec<C>,
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
