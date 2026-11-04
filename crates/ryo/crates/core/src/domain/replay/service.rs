//! The `replay` service: free functions named by verb that pass the rows of a step on, one by one.

use ryo_relations::domain::relations::types::Value;

use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, OperatorContext};
use crate::domain::table_model::service::{count_of, positions, read_row};
use crate::domain::table_model::types::Rows;

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
    if matches!(rows, Rows::Computed(_) | Rows::Compact(_)) {
        crate::domain::table_model::collections::representations::service::borrowed(rows)
            .validate()?;
    }
    for position in positions(count_of(rows)) {
        if let Some(row) = read_row(rows, position, buffer) {
            emit.row(context, row)?;
        } else if matches!(rows, Rows::Computed(_) | Rows::Compact(_)) {
            return Err(OperatingError::ShapeMismatch);
        }
    }
    Ok(())
}
