//! The engine chooses the physical probe; this kernel owns ordered Join row semantics.
use crate::domain::contract::types::{JoinDriver, PortableOperatorFault};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};

pub(crate) fn join(
    left: &[Value],
    left_on: &[ColumnIndex],
    output: &[JoinColumn],
    view: &[ColumnIndex],
    driver: &mut dyn JoinDriver,
    key: &mut Vec<Value>,
    target: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    key.clear();
    for column in left_on {
        key.push(
            *left
                .get(column.0 as usize)
                .ok_or(PortableOperatorFault::NoSuchColumn { index: column.0 })?,
        );
    }
    while driver
        .next(key)
        .map_err(|_| PortableOperatorFault::Interrupted)?
    {
        target.clear();
        for column in output {
            let value = match column.side {
                Side::Left => left.get(column.column.0 as usize).copied(),
                Side::Right => driver.right_cell(
                    view.get(column.column.0 as usize)
                        .copied()
                        .unwrap_or(column.column),
                ),
            }
            .ok_or(PortableOperatorFault::NoSuchColumn {
                index: column.column.0,
            })?;
            target.push(value);
        }
        driver
            .emit(target)
            .map_err(|_| PortableOperatorFault::Interrupted)?;
    }
    Ok(())
}

#[cfg(test)]
mod service_test;

/// A lowered row's schema already maps logical columns onto these slots. Keep
/// them deferred: only demanded key/filter/output reads may decode cells.
pub(crate) fn positions(
    left: &[u32],
    driver: &mut dyn crate::domain::contract::types::PositionJoinDriver,
    target: &mut Vec<u32>,
) -> Result<(), PortableOperatorFault> {
    while driver
        .next()
        .map_err(|_| PortableOperatorFault::Interrupted)?
    {
        target.extend_from_slice(left);
        target.extend_from_slice(driver.right_positions());
    }
    Ok(())
}
