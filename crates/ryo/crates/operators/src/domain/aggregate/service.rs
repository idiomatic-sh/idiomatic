//! Format a group without taking ownership of reduction state or first-row acquisition.
use crate::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::types::{Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
pub(crate) fn aggregate(
    row: &[Value],
    keys: &[ColumnIndex],
    count: usize,
    digest: Option<i64>,
    target: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    target.clear();
    crate::domain::project::service::project(row, keys, target)?;
    if let Some(digest) = digest {
        target.push(Value::Number(Integer(digest)));
    }
    target.push(Value::Number(Integer(
        i64::try_from(count).unwrap_or(i64::MAX),
    )));
    Ok(())
}
