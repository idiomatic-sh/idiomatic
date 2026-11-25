//! The `project` item of the `kernels` collection: a row's columns in order, duplicates kept, through the projection's one
//! meaning; a repeated row dropped by `distinct` when asked.

use super::super::distinct::item::counted;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::CellStart;
use crate::domain::physical_plan::types::row_set::value::RowSet;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, RepeatedRows};
use ryo_values::domain::rows::types::Table;



/// Appends a row's projection to `output`, the first time only when repeated rows are dropped.
///
/// # Errors
///
/// `NoSuchColumn` for a column past the row.
pub(crate) fn projected(
    row: &[Value],
    columns: &[ColumnIndex],
    distinct: RepeatedRows,
    repeats: &mut RowSet,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    let start = CellStart(output.cells.len());
    crate::domain::project::service::project(row, columns, &mut output.cells)?;
    counted(output, start, distinct, repeats);
    Ok(())
}

/// Spike (ryo-fmx3): `projected` over row `at` of a relation's columns, each projected cell read in place.
///
/// # Errors
///
/// `NoSuchColumn` for a column past the row, the cells before it written, as `projected` leaves them.
pub(crate) fn projected_facts(
    view: &ryo_relations::domain::relations::types::ColumnsView<'_>,
    at: ryo_values::domain::rows::types::RowIndex,
    columns: &[ColumnIndex],
    distinct: RepeatedRows,
    repeats: &mut RowSet,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    let start = CellStart(output.cells.len());
    for column in columns {
        output.cells.push(
            super::super::scan_facts::item::facts_cell_at(
                view,
                at,
                usize::try_from(column.0).unwrap_or(usize::MAX),
            )
            .ok_or(PortableOperatorFault::NoSuchColumn { index: *column })?,
        );
    }
    counted(output, start, distinct, repeats);
    Ok(())
}
