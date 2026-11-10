//! The `replay` service: free functions named by verb that pass the rows of a step on, one by one.

use ryo_relations::domain::relations::types::Value;

use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::types::{Emit, OperatorContext};
use crate::domain::table_model::service::{count_of, read_row};
use crate::domain::table_model::types::Rows;
use ryo_values::domain::rows::service::positions;

/// Passes on the rows of `rows`, one by one, as they are in the store or in a table.
///
/// # Errors
///
/// Whatever `emit` refuses.
pub fn scan(
    rows: &Rows<'_>,
    buffer: &mut Vec<Value>,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    if matches!(rows, Rows::Computed(_) | Rows::Compact(_)) {
        crate::domain::table_model::service::borrowed(rows).validate()?;
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

/// Deliver a selection's accepted prefix before its terminal predicate error.
/// The existing continuation decides downstream semantics and sink refusal order.
///
/// # Errors
///
/// [`OperatingError::NotRead`] when a selected position or a kept column is not in the source, then the first error
/// the continuation returns.
pub fn selected(
    result: &crate::domain::shared_walk::types::ClassResult,
    source: &Rows<'_>,
    kept: super::types::SelectionColumns<'_>,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let mut cells = Vec::new();
    let mut projected = Vec::new();
    for position in &result.positions.0 {
        let Ok(position) = usize::try_from(*position) else {
            return Err(OperatingError::NotRead);
        };
        let row = read_row(
            source,
            ryo_values::domain::rows::types::RowIndex(position),
            &mut cells,
        )
        .ok_or(OperatingError::NotRead)?;
        let row = match kept {
            super::types::SelectionColumns::All => row,
            super::types::SelectionColumns::Columns(columns) => {
                projected.clear();
                for column in columns {
                    let Ok(at) = usize::try_from(column.0) else {
                        return Err(OperatingError::NotRead);
                    };
                    projected.push(*row.get(at).ok_or(OperatingError::NotRead)?);
                }
                &projected
            }
        };
        emit.row(context, row)?;
    }
    result.completion.clone()
}
