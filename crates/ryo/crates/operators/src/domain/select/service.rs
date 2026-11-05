//! Predicate outcome precedes kept-column reads, including malformed unused columns.
use crate::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
pub(crate) fn select(
    row: &[Value],
    kept: &[ColumnIndex],
    holds: bool,
    target: &mut Vec<Value>,
) -> Result<bool, PortableOperatorFault> {
    if !holds {
        return Ok(false);
    }
    if !kept.is_empty() {
        target.clear();
        crate::domain::project::service::project(row, kept, target)?;
    }
    Ok(true)
}
