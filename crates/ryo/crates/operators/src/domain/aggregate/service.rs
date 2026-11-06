//! Format a group without taking ownership of reduction state or first-row acquisition.
use crate::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::types::{Count, Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
pub(crate) fn aggregate(
    row: &[Value],
    keys: &[ColumnIndex],
    count: Count,
    digest: Option<Integer>,
    target: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    target.clear();
    crate::domain::project::service::project(row, keys, target)?;
    if let Some(digest) = digest {
        target.push(Value::Number(digest));
    }
    target.push(Value::Number(Integer(
        i64::try_from(count.0).unwrap_or(i64::MAX),
    )));
    Ok(())
}
