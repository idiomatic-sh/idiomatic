//! PredicateOutcome outcome precedes kept-column reads, including malformed unused columns.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::select::types::{PredicateOutcome, RowSelection};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
pub(crate) fn select(
    row: &[Value],
    kept: &[ColumnIndex],
    holds: PredicateOutcome,
    target: &mut Vec<Value>,
) -> Result<RowSelection, PortableOperatorFault> {
    if holds == PredicateOutcome::Fails {
        return Ok(RowSelection::Dropped);
    }
    if !kept.is_empty() {
        target.clear();
        crate::domain::project::service::project(row, kept, target)?;
    }
    Ok(RowSelection::Kept)
}

/// Keeps a scheduled row its bound predicate holds for, with the authored kept columns; an empty projection
/// keeps the original row.
pub(crate) fn evaluate(
    row: &[Value],
    kept: &[ColumnIndex],
    driver: &mut dyn crate::domain::evaluation::types::InvocationDriver,
    projected: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    let holds = driver.holds(row)?;
    match select(row, kept, holds, projected)? {
        RowSelection::Dropped => Ok(()),
        RowSelection::Kept => driver.emit(if kept.is_empty() { row } else { projected }),
    }
}
