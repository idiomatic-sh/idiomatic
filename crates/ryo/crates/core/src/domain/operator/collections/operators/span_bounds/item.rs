//! Project checked byte endpoints of a typed span without changing row order or bag.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::service::cell;
use crate::domain::table_model::types::ParameterBinding;
use ryo_language::domain::relations::constants::NO_SPAN;
use ryo_language::domain::relations::types::{Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plans::types::{Operation, OperationKind};
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
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputSchema<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::SpanBounds,
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
        let Operation::SpanBounds { span, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let projected = bounds(row, *span)?;
        let mut joined = std::mem::take(lease.workspace.row);
        joined.clear();
        joined.extend_from_slice(row);
        joined.extend(projected);
        let result = emit.row(lease, &joined);
        *lease.workspace.row = joined;
        result
    }
}
