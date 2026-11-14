//! FIFO pairing over already prepared, ordered populations.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::ordered_match::types::{KeyOrder, OrderedMatchDriver};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
use ryo_values::domain::rows::types::RowIndex;
pub(crate) fn pairs(
    output: &[JoinColumn],
    driver: &mut dyn OrderedMatchDriver,
    target: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    let mut next_right = 0;
    for left in (0..driver.left_len().0).map(RowIndex) {
        while next_right < driver.right_len().0 {
            let keys = driver.compare_keys(left, RowIndex(next_right));
            if keys.is_gt()
                || keys.is_eq()
                    && driver.right_order_before_or_equal(left, RowIndex(next_right))
                        == KeyOrder::BeforeOrEqual
            {
                next_right += 1;
            } else {
                break;
            }
        }
        if next_right >= driver.right_len().0 {
            break;
        }
        if !driver.compare_keys(left, RowIndex(next_right)).is_eq() {
            continue;
        }
        driver.visit_pair(left, RowIndex(next_right), &mut |left, right| {
            target.clear();
            for column in output {
                let row = match column.side {
                    Side::Left => left,
                    Side::Right => right,
                };
                let index = usize::try_from(column.column.0).map_err(|_| {
                    PortableOperatorFault::NoSuchColumn {
                        index: column.column,
                    }
                })?;
                target.push(*row.get(index).ok_or(PortableOperatorFault::NoSuchColumn {
                    index: column.column,
                })?);
            }
            Ok(())
        })?;
        driver
            .emit(target)
            .map_err(|_| PortableOperatorFault::Interrupted)?;
        next_right += 1;
    }
    Ok(())
}

/// Pairs the engine's prepared ordered keys of both inputs in FIFO order.
pub(crate) fn evaluate(
    output: &[JoinColumn],
    driver: &mut dyn crate::domain::ordered_match::types::OrderedMatchEvaluation,
    target: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    pairs(output, driver.ordered_match()?, target)
}

/// The one lowering of an ordered match: both sides' keys and orders and its output columns, for the `ordered_match` kernel.
pub(crate) fn lower(
    left_on: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    right_on: &[ryo_values::domain::plan_parts::types::ColumnIndex],
    left_order: ryo_values::domain::plan_parts::types::ColumnIndex,
    right_order: ryo_values::domain::plan_parts::types::ColumnIndex,
    output: &[JoinColumn],
) -> super::types::OrderedParts {
    super::types::OrderedParts {
        left_on: left_on.to_vec(),
        right_on: right_on.to_vec(),
        left_order,
        right_order,
        output: output.to_vec(),
    }
}

/// Pairs both sides of `ordered`, read from `left` and `right`: each left row, in key then order, with the earliest right row
/// of an equal key whose order is after its own, first in first out, each right row paired once; each pair's output columns
/// are written through `emit`.
///
/// # Errors
///
/// `NoSuchColumn` for an output column past its row; `ShapeMismatch` for a row the ordering names that its side cannot read;
/// the fault `emit` answers.
pub(crate) fn paired(
    output: &[JoinColumn],
    ordered: &ryo_derived::domain::ordered::types::OrderedMatching,
    left: &dyn ryo_derived::domain::row_cells::types::RowCells,
    right: &dyn ryo_derived::domain::row_cells::types::RowCells,
    emit: &mut dyn FnMut(&[Value]) -> Result<(), PortableOperatorFault>,
) -> Result<(), PortableOperatorFault> {
    let (mut left_row, mut right_row, mut target) = (Vec::new(), Vec::new(), Vec::new());
    let mut next_right = 0;
    for left_at in &ordered.left {
        while let Some(right_at) = ordered.right.get(next_right) {
            let keys = left_at.keys.cmp(&right_at.keys);
            if keys.is_gt() || keys.is_eq() && right_at.order <= left_at.order {
                next_right += 1;
            } else {
                break;
            }
        }
        let Some(right_at) = ordered.right.get(next_right) else {
            break;
        };
        if left_at.keys != right_at.keys {
            continue;
        }
        let left_cells = left
            .read_row(left_at.row, &mut left_row)
            .ok_or(PortableOperatorFault::ShapeMismatch)?;
        let right_cells = right
            .read_row(right_at.row, &mut right_row)
            .ok_or(PortableOperatorFault::ShapeMismatch)?;
        target.clear();
        for column in output {
            let row = match column.side {
                Side::Left => left_cells,
                Side::Right => right_cells,
            };
            target.push(
                *row.get(usize::try_from(column.column.0).unwrap_or(usize::MAX))
                    .ok_or(PortableOperatorFault::NoSuchColumn {
                        index: column.column,
                    })?,
            );
        }
        emit(&target)?;
        next_right += 1;
    }
    Ok(())
}
