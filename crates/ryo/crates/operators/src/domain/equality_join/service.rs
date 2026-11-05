//! The engine chooses the physical probe; this kernel owns ordered Join row semantics.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::equality_join::types::{JoinDriver, JoinRow};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};

/// Ordered Join meaning over the engine-selected borrowed left representation.
pub(crate) fn join_row(
    left: JoinRow<'_>,
    left_on: &[ColumnIndex],
    output: &[JoinColumn],
    view: &[ColumnIndex],
    driver: &mut dyn JoinDriver,
    key: &mut Vec<Value>,
    target: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    let read_left = |column: ColumnIndex| match &left {
        JoinRow::Values(row) => usize::try_from(column.0)
            .ok()
            .and_then(|at| row.get(at))
            .copied(),
        JoinRow::Columns(row) => row.cell(column),
    };
    key.clear();
    for column in left_on {
        key.push(
            read_left(*column).ok_or(PortableOperatorFault::NoSuchColumn { index: column.0 })?,
        );
    }
    while driver
        .next(key)
        .map_err(|_| PortableOperatorFault::Interrupted)?
    {
        target.clear();
        for column in output {
            let value = match column.side {
                Side::Left => read_left(column.column),
                Side::Right => driver.right_cell(
                    view.get(usize::try_from(column.column.0).unwrap_or(usize::MAX))
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
    driver: &mut dyn crate::domain::equality_join::types::PositionJoinDriver,
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

#[cfg(test)]
fn join(
    left: &[Value],
    left_on: &[ColumnIndex],
    output: &[JoinColumn],
    view: &[ColumnIndex],
    driver: &mut dyn JoinDriver,
    key: &mut Vec<Value>,
    target: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    join_row(
        JoinRow::Values(left),
        left_on,
        output,
        view,
        driver,
        key,
        target,
    )
}
