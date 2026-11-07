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

/// Formats each engine-prepared group of the contributing population from its first row, in group order.
/// A counted group is the step's input count; a digested group's rows were counted by its reduction.
pub(crate) fn evaluate(
    keys: &[ColumnIndex],
    aggregation: super::types::Aggregation,
    driver: &mut dyn crate::domain::contract::types::RowDriver,
    scratch: &mut crate::domain::contract::types::RowScratch,
) -> Result<(), PortableOperatorFault> {
    use crate::domain::contract::types::RowRead;
    use crate::domain::contract::types::VertexPlace;
    use ryo_values::domain::plan_parts::types::InputNumber;
    let groups = driver.groups()?;
    for at in 0..groups.0 {
        let group = driver
            .group(VertexPlace(at))
            .ok_or(PortableOperatorFault::ShapeMismatch)?;
        if driver.population_row(InputNumber(0), group.first, &mut scratch.row) != RowRead::Read {
            return Err(PortableOperatorFault::NotRun);
        }
        let digest = match aggregation {
            super::types::Aggregation::Counted => {
                driver.found(InputNumber(0), group.counted);
                None
            }
            super::types::Aggregation::Digested => Some(group.digest),
        };
        aggregate(
            &scratch.row,
            keys,
            group.counted,
            digest,
            &mut scratch.other,
        )?;
        driver.emit(&scratch.other)?;
    }
    Ok(())
}
