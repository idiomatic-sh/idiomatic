//! Project checked byte endpoints of a typed span without changing row order or bag.

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::first;
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room};
use crate::domain::replay::types::Pipe;
use crate::domain::table_model::service::{cell, count_of, kinds_of, positions, read_row, reset};
use crate::domain::table_model::types::{Rows, Table};
use ryo_language::domain::relations::constants::NO_SPAN;
use ryo_language::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plans::types::Operation;

fn bounds(row: &[Value], column: ColumnIndex) -> Result<[Value; 2], OperatingError> {
    let Value::Span(span) = cell(row, column)? else {
        return Err(OperatingError::ShapeMismatch);
    };
    if span == NO_SPAN || span.start > span.end {
        return Err(OperatingError::InvalidSpanBounds);
    }
    let start = usize::try_from(span.start.0).map_err(|_| OperatingError::InvalidSpanBounds)?;
    let end = usize::try_from(span.end.0).map_err(|_| OperatingError::InvalidSpanBounds)?;
    let start = i64::try_from(start).map_err(|_| OperatingError::InvalidSpanBounds)?;
    let end = i64::try_from(end).map_err(|_| OperatingError::InvalidSpanBounds)?;
    Ok([Value::Number(Integer(start)), Value::Number(Integer(end))])
}

pub(crate) struct SpanBoundsItem;

impl OperatorsItem for SpanBoundsItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::SpanBounds { span, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let input = first(inputs)?;
        reset(out, kinds_of(input));
        out.kinds.extend([CellKind::Number, CellKind::Number]);
        let mut buffer = std::mem::take(&mut context.scratch.row);
        let result = (|| {
            for at in positions(count_of(input)) {
                let Some(row) = read_row(input, at, &mut buffer) else {
                    continue;
                };
                let projected = bounds(row, *span)?;
                out.cells.extend_from_slice(row);
                out.cells.extend(projected);
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
        let Operation::SpanBounds { span, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let mut joined = std::mem::take(&mut room.scratch.row);
        let result = ins.stream(
            InputPlace(0),
            context,
            &mut Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
                let projected = bounds(row, *span)?;
                joined.clear();
                joined.extend_from_slice(row);
                joined.extend(projected);
                emit.row(each, &joined)
            }),
        );
        room.scratch.row = joined;
        result
    }
}
