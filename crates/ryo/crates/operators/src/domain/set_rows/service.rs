//! Set policy controls membership, distinctness, laziness and emission order.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::set_rows::types::{SetKind, SetPositionsDriver, SetRowDriver};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// Apply one scheduled typed row. The engine retains the acquired membership and scratch.
pub(crate) fn values(
    kind: SetKind,
    row: &[Value],
    left_on: &[ColumnIndex],
    driver: &mut dyn SetRowDriver,
    key: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    let keep = match kind {
        SetKind::Union => true,
        SetKind::Intersection => driver
            .contains_right(row)
            .map_err(|_| PortableOperatorFault::Interrupted)?,
        SetKind::Difference => !driver
            .contains_right(row)
            .map_err(|_| PortableOperatorFault::Interrupted)?,
        SetKind::Antijoin => {
            if driver
                .right_empty()
                .map_err(|_| PortableOperatorFault::Interrupted)?
            {
                true
            } else {
                key.clear();
                for column in left_on {
                    let at = usize::try_from(column.0)
                        .map_err(|_| PortableOperatorFault::NoSuchColumn { index: column.0 })?;
                    key.push(
                        row.get(at)
                            .copied()
                            .ok_or(PortableOperatorFault::NoSuchColumn { index: column.0 })?,
                    );
                }
                !driver
                    .contains_right(key)
                    .map_err(|_| PortableOperatorFault::Interrupted)?
            }
        }
    };
    if !keep {
        return Ok(());
    }
    let first = match kind {
        SetKind::Antijoin => true,
        _ => driver
            .first_time(row)
            .map_err(|_| PortableOperatorFault::Interrupted)?,
    };
    if first {
        driver
            .emit(row)
            .map_err(|_| PortableOperatorFault::Interrupted)?;
    }
    Ok(())
}

/// Apply set policy to deferred position rows in encounter order.
pub(crate) fn positions(
    kind: SetKind,
    driver: &mut dyn SetPositionsDriver,
) -> Result<(), PortableOperatorFault> {
    while driver
        .next()
        .map_err(|_| PortableOperatorFault::Interrupted)?
    {
        let keep = match kind {
            SetKind::Union => true,
            SetKind::Intersection => driver
                .contains_right()
                .map_err(|_| PortableOperatorFault::Interrupted)?,
            SetKind::Difference => !driver
                .contains_right()
                .map_err(|_| PortableOperatorFault::Interrupted)?,
            SetKind::Antijoin => {
                driver
                    .right_empty()
                    .map_err(|_| PortableOperatorFault::Interrupted)?
                    || !driver
                        .contains_right()
                        .map_err(|_| PortableOperatorFault::Interrupted)?
            }
        };
        if !keep {
            continue;
        }
        let first = match kind {
            SetKind::Antijoin => true,
            _ => driver
                .first_time()
                .map_err(|_| PortableOperatorFault::Interrupted)?,
        };
        if first {
            driver
                .emit()
                .map_err(|_| PortableOperatorFault::Interrupted)?;
        }
    }
    Ok(())
}
