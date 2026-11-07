//! Each admitted candidate of an axis walk is one row, in walk order; the nearest ancestor is the first.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::evaluation::service::cell;
use crate::domain::evaluation::types::{Admission, AxisCandidates, InvocationDriver, VertexPlace};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction, JoinColumn, Side};

/// Emits a scheduled row once for each candidate its axis walk admits: the row with the entity or node
/// appended, or the declared output over the row and the element. A cached complete answer is emitted as it
/// is.
pub(crate) fn evaluate(
    row: &[Value],
    id: ColumnIndex,
    direction: Direction,
    output: &[JoinColumn],
    cached: Option<&[Value]>,
    driver: &mut dyn InvocationDriver,
    answer: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    if let Some(destinations) = cached {
        for destination in destinations {
            answer.clear();
            answer.extend_from_slice(row);
            answer.push(*destination);
            driver.emit(answer)?;
        }
        return Ok(());
    }
    let (count, elements) = match driver.axis().walk(direction, row, id)? {
        AxisCandidates::Entities(count) => (count, false),
        AxisCandidates::Elements(count) => (count, true),
    };
    for at in (0..count.0).map(VertexPlace) {
        if driver.axis().admits(at)? == Admission::Out {
            continue;
        }
        answer.clear();
        if !elements || output.is_empty() {
            answer.extend_from_slice(row);
            answer.push(
                driver
                    .axis()
                    .candidate(at)
                    .ok_or(PortableOperatorFault::ShapeMismatch)?,
            );
        } else {
            for column in output {
                answer.push(match column.side {
                    Side::Left => cell(row, column.column)?,
                    Side::Right => driver
                        .axis()
                        .element_cell(at, column.column)
                        .ok_or(PortableOperatorFault::ShapeMismatch)?,
                });
            }
        }
        driver.emit(answer)?;
        if direction == Direction::NearestAncestor {
            break;
        }
    }
    Ok(())
}
