//! Append a checked half-open byte interval without changing input rows.

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::first;
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room};
use crate::domain::replay::types::Pipe;
use crate::domain::table_model::service::{cell, count_of, kinds_of, positions, read_row, reset};
use crate::domain::table_model::types::{Rows, Table};
use ryo_language::domain::relations::constants::NO_SPAN;
use ryo_language::domain::relations::types::{CellKind, Span, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plans::types::Operation;

/// Validate both located operands even when the other operand is absent.
fn composed(
    row: &[Value],
    start: ColumnIndex,
    end: ColumnIndex,
    length: usize,
) -> Result<Value, OperatingError> {
    let (Value::Span(left), Value::Span(right)) = (cell(row, start)?, cell(row, end)?) else {
        return Err(OperatingError::ShapeMismatch);
    };
    for span in [left, right] {
        if span != NO_SPAN {
            let end = usize::try_from(span.end.0).map_err(|_| OperatingError::InvalidSpan)?;
            if span.start > span.end || end > length {
                return Err(OperatingError::InvalidSpan);
            }
        }
    }
    if left == NO_SPAN || right == NO_SPAN {
        return Ok(Value::Span(NO_SPAN));
    }
    if left.start > right.end {
        return Err(OperatingError::InvalidSpan);
    }
    Ok(Value::Span(Span {
        start: left.start,
        end: right.end,
    }))
}

/// The per-file span projection.
pub(crate) struct SpanBetweenItem;

impl OperatorsItem for SpanBetweenItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::SpanBetween { start, end, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let input = first(inputs)?;
        reset(out, kinds_of(input));
        out.kinds.push(CellKind::Span);
        let mut buffer = std::mem::take(&mut context.scratch.row);
        let result = (|| {
            for at in positions(count_of(input)) {
                let Some(row) = read_row(input, at, &mut buffer) else {
                    continue;
                };
                let value = composed(row, *start, *end, context.source.0.len())?;
                out.cells.extend_from_slice(row);
                out.cells.push(value);
                out.count.0 += 1;
            }
            Ok(())
        })();
        context.scratch.row = buffer;
        result
    }

    fn stream(
        &self,
        operation: &Operation,
        ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let Operation::SpanBetween { start, end, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let mut joined = std::mem::take(&mut room.scratch.row);
        let result = ins.stream(
            InputPlace(0),
            context,
            &mut Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
                let value = composed(row, *start, *end, each.source.0.len())?;
                joined.clear();
                joined.extend_from_slice(row);
                joined.push(value);
                emit.row(each, &joined)
            }),
        );
        room.scratch.row = joined;
        result
    }
}
