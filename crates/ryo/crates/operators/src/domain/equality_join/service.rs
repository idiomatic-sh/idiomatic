//! The engine chooses the physical probe; this kernel owns ordered Join row semantics.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::equality_join::types::{JoinDriver, JoinRow, PositionSlots, PositionTarget};
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
        JoinRow::Values(row) => {
            let Ok(at) = usize::try_from(column.0) else {
                return None;
            };
            row.get(at).copied()
        }
        JoinRow::Columns(row) => row.cell(column),
    };
    key.clear();
    for column in left_on {
        key.push(
            read_left(*column).ok_or(PortableOperatorFault::NoSuchColumn { index: column })?,
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
                    match view.get(usize::try_from(column.column.0).unwrap_or(usize::MAX)) {
                        Some(physical) => *physical,
                        None => column.column,
                    },
                ),
            }
            .ok_or(PortableOperatorFault::NoSuchColumn {
                index: column.column,
            })?;
            target.push(value);
        }
        driver
            .emit(target)
            .map_err(|_| PortableOperatorFault::Interrupted)?;
    }
    Ok(())
}

/// A lowered row's schema already maps logical columns onto these slots. Keep
/// them deferred: only demanded key/filter/output reads may decode cells.
pub(crate) fn positions(
    left: PositionSlots<'_>,
    driver: &mut dyn crate::domain::equality_join::types::PositionJoinDriver,
    target: PositionTarget<'_>,
) -> Result<(), PortableOperatorFault> {
    let (left, target) = (left.0, target.0);
    while driver
        .next()
        .map_err(|_| PortableOperatorFault::Interrupted)?
    {
        target.extend_from_slice(left);
        target.extend_from_slice(driver.right_positions());
    }
    Ok(())
}
