//! The `replay` service: free functions named by verb that pass the rows of a step on, one by one.

use ryo_relations::domain::relations::types::Value;

use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, OperatorContext};
use crate::domain::table_model::service::{count_of, read_row};
use crate::domain::table_model::types::Rows;
use ryo_values::domain::rows::service::positions;

/// Passes on the rows of `rows`, one by one, as they are in the store or in a table.
///
/// # Errors
///
/// Whatever `emit` refuses.
pub(crate) fn scan(
    rows: &Rows<'_>,
    buffer: &mut Vec<Value>,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    if matches!(
        rows,
        Rows::Computed(_) | Rows::Compact(_) | Rows::Positioned(_)
    ) {
        crate::domain::table_model::collections::representations::service::borrowed(rows)
            .validate()?;
    }
    for position in positions(count_of(rows)) {
        if let Some(row) = read_row(rows, position, buffer) {
            emit.row(context, row)?;
        } else if matches!(
            rows,
            Rows::Computed(_) | Rows::Compact(_) | Rows::Positioned(_)
        ) {
            return Err(OperatingError::ShapeMismatch);
        }
    }
    Ok(())
}

/// Deliver a selection's accepted prefix before its terminal predicate error.
/// The existing continuation decides downstream semantics and sink refusal order.
pub(crate) fn selected(
    result: &crate::domain::shared_walk::types::ClassResult,
    source: &Rows<'_>,
    kept: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let mut cells = Vec::new();
    let mut projected = Vec::new();
    for position in &result.positions.0 {
        let row = read_row(
            source,
            ryo_values::domain::rows::types::RowIndex(*position as usize),
            &mut cells,
        )
        .ok_or(OperatingError::NotRead)?;
        let row = if kept.is_empty() {
            row
        } else {
            projected.clear();
            for column in kept {
                projected.push(*row.get(column.0 as usize).ok_or(OperatingError::NotRead)?);
            }
            &projected
        };
        emit.row(context, row)?;
    }
    result.completion.clone()
}
