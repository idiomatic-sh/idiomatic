//! FIFO pairing over already prepared, ordered populations.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::ordered_match::types::{KeyOrder, OrderedMatchDriver};
use ryo_values::domain::rows::types::RowIndex;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
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
