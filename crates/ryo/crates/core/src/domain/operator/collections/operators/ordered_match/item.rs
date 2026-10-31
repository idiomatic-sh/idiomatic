//! Deterministic FIFO matching of equally keyed ordered rows.

use super::super::interface::OperatorsItem;
use super::super::types::{OrderedKey, OrderedRow};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::{first, second};
use crate::domain::operating::types::{Emit, Feed, OperatorContext, Room};
use crate::domain::replay::service::replay_computed;
use crate::domain::table_model::service::{cell, count_of, kinds_of, positions, read_row, reset};
use crate::domain::table_model::types::{Rows, Table};
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_language::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Side};
use ryo_values::domain::plans::types::Operation;

/// One ordered matching operation.
pub(crate) struct OrderedMatchItem;

/// Index rows once, preserving text equality across source and retained arenas.
fn indexed<'a>(
    input: &Rows<'_>,
    keys: &[ColumnIndex],
    order: ColumnIndex,
    texts: &Texts<'a>,
) -> Result<Vec<OrderedRow<'a>>, OperatingError> {
    let mut rows = Vec::with_capacity(count_of(input).0);
    let mut buffer = Vec::new();
    for at in positions(count_of(input)) {
        let Some(row) = read_row(input, at, &mut buffer) else {
            continue;
        };
        let Value::Number(order) = cell(row, order)? else {
            return Err(OperatingError::ShapeMismatch);
        };
        let mut group = Vec::with_capacity(keys.len());
        for key in keys {
            let value = cell(row, *key)?;
            group.push(match value {
                Value::Symbol(_) | Value::Name(_) => {
                    let Datum::Text(text) = datum(value, texts) else {
                        return Err(OperatingError::ShapeMismatch);
                    };
                    OrderedKey::Text(text)
                }
                Value::Node(_)
                | Value::File(_)
                | Value::Span(_)
                | Value::Number(_)
                | Value::Flag(_) => OrderedKey::Value(value),
            });
        }
        rows.push(OrderedRow {
            keys: group,
            order,
            row: at,
        });
    }
    rows.sort_unstable();
    if rows
        .windows(2)
        .any(|pair| pair[0].keys == pair[1].keys && pair[0].order == pair[1].order)
    {
        return Err(OperatingError::DuplicateOrder);
    }
    Ok(rows)
}

impl OperatorsItem for OrderedMatchItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::OrderedMatch {
            left_on,
            right_on,
            left_order,
            right_order,
            output,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let left = first(inputs)?;
        let right = second(inputs)?;
        if left_on.len() != right_on.len()
            || kinds_of(left).get(left_order.0 as usize) != Some(&CellKind::Number)
            || kinds_of(right).get(right_order.0 as usize) != Some(&CellKind::Number)
        {
            return Err(OperatingError::ShapeMismatch);
        }
        for (left_key, right_key) in left_on.iter().zip(right_on) {
            if kinds_of(left).get(left_key.0 as usize) != kinds_of(right).get(right_key.0 as usize)
            {
                return Err(OperatingError::ShapeMismatch);
            }
        }
        let texts = Texts {
            symbols: context.symbols,
            source: context.source,
            paths: context.paths,
            arenas: context.arenas,
        };
        let indexed_left = indexed(left, left_on, *left_order, &texts)?;
        let indexed_right = indexed(right, right_on, *right_order, &texts)?;
        reset(out, &[]);
        for column in output {
            let kinds = match column.side {
                Side::Left => kinds_of(left),
                Side::Right => kinds_of(right),
            };
            out.kinds.push(
                *kinds
                    .get(column.column.0 as usize)
                    .ok_or(OperatingError::ShapeMismatch)?,
            );
        }
        let mut left_buffer = Vec::new();
        let mut right_buffer = Vec::new();
        let mut next_right = 0;
        for opened in indexed_left {
            while let Some(closed) = indexed_right.get(next_right) {
                if closed.keys < opened.keys
                    || closed.keys == opened.keys && closed.order <= opened.order
                {
                    next_right += 1;
                } else {
                    break;
                }
            }
            let Some(closed) = indexed_right.get(next_right) else {
                break;
            };
            if closed.keys != opened.keys {
                continue;
            }
            let left_row = read_row(left, opened.row, &mut left_buffer)
                .ok_or(OperatingError::ShapeMismatch)?;
            let right_row = read_row(right, closed.row, &mut right_buffer)
                .ok_or(OperatingError::ShapeMismatch)?;
            for column in output {
                out.cells.push(cell(
                    match column.side {
                        Side::Left => left_row,
                        Side::Right => right_row,
                    },
                    column.column,
                )?);
            }
            out.count.0 += 1;
            next_right += 1;
        }
        Ok(())
    }

    fn stream(
        &self,
        operation: &Operation,
        ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        replay_computed(
            operation,
            ins,
            context,
            room,
            emit,
            |operation, inputs, context, out| {
                OrderedMatchItem.compute(operation, inputs, context, out)
            },
        )
    }
}
