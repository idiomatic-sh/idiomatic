//! Set policy controls membership, distinctness, laziness and emission order.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::equality_join::types::Advance;
use crate::domain::set_rows::types::{
    Membership, Occurrence, Population, SetKind, SetPositionsDriver, SetRowDriver,
};
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
        SetKind::Intersection => {
            driver
                .contains_right(row)
                .map_err(|_| PortableOperatorFault::Interrupted)?
                == Membership::Present
        }
        SetKind::Difference => {
            driver
                .contains_right(row)
                .map_err(|_| PortableOperatorFault::Interrupted)?
                == Membership::Absent
        }
        SetKind::Antijoin => {
            if driver
                .right_empty()
                .map_err(|_| PortableOperatorFault::Interrupted)?
                == Population::Empty
            {
                true
            } else {
                key.clear();
                for column in left_on {
                    let at = usize::try_from(column.0)
                        .map_err(|_| PortableOperatorFault::NoSuchColumn { index: column })?;
                    key.push(
                        row.get(at)
                            .copied()
                            .ok_or(PortableOperatorFault::NoSuchColumn { index: column })?,
                    );
                }
                driver
                    .contains_right(key)
                    .map_err(|_| PortableOperatorFault::Interrupted)?
                    == Membership::Absent
            }
        }
    };
    if !keep {
        return Ok(());
    }
    let first = match kind {
        SetKind::Antijoin => true,
        _ => {
            driver
                .first_time(row)
                .map_err(|_| PortableOperatorFault::Interrupted)?
                == Occurrence::First
        }
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
        == Advance::Admitted
    {
        let keep = match kind {
            SetKind::Union => true,
            SetKind::Intersection => {
                driver
                    .contains_right()
                    .map_err(|_| PortableOperatorFault::Interrupted)?
                    == Membership::Present
            }
            SetKind::Difference => {
                driver
                    .contains_right()
                    .map_err(|_| PortableOperatorFault::Interrupted)?
                    == Membership::Absent
            }
            SetKind::Antijoin => {
                driver
                    .right_empty()
                    .map_err(|_| PortableOperatorFault::Interrupted)?
                    == Population::Empty
                    || driver
                        .contains_right()
                        .map_err(|_| PortableOperatorFault::Interrupted)?
                        == Membership::Absent
            }
        };
        if !keep {
            continue;
        }
        let first = match kind {
            SetKind::Antijoin => true,
            _ => {
                driver
                    .first_time()
                    .map_err(|_| PortableOperatorFault::Interrupted)?
                    == Occurrence::First
            }
        };
        if first {
            driver
                .emit()
                .map_err(|_| PortableOperatorFault::Interrupted)?;
        }
    }
    Ok(())
}
