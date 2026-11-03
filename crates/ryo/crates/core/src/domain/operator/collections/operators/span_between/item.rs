//! Append a checked half-open byte interval without changing input rows.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::service::cell;
use crate::domain::table_model::types::ParameterBinding;
use ryo_relations::domain::relations::constants::NO_SPAN;
use ryo_relations::domain::relations::types::{Span, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plans::types::{Operation, OperationKind};
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
            let span_end =
                usize::try_from(span.end.0).map_err(|_error| OperatingError::InvalidSpan)?;
            if span.start > span.end || span_end > length {
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

pub(crate) struct SpanBetweenItem;
impl OperatorsItem for SpanBetweenItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputSchema<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::SpanBetween,
            operation,
            inputs,
            bindings,
        )
    }
    fn evaluate(
        &self,
        operation: &Operation,
        inputs: PreparedInputs<'_>,
        lease: &mut PrimitiveLease<'_>,
        emit: &mut dyn RowSink,
    ) -> Result<(), OperatingError> {
        let PreparedInputs::Row { row, .. } = inputs else {
            return Err(OperatingError::WrongOperation);
        };
        let Operation::SpanBetween { start, end, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let projected = composed(row, *start, *end, lease.views.source.0.len())?;
        let mut joined = std::mem::take(lease.workspace.row);
        joined.clear();
        joined.extend_from_slice(row);
        joined.push(projected);
        let result = emit.row(lease, &joined);
        *lease.workspace.row = joined;
        result
    }
}
