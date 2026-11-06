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
