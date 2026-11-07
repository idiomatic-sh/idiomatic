//! Append a checked half-open byte interval without changing input rows.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_operators::types::OperationKind;
pub(crate) struct SpanBetweenItem;
impl OperatorsItem for SpanBetweenItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
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
        let projected = lease.span_between(row, *start, *end, lease.views.source.0.len())?;
        let mut joined = std::mem::take(&mut lease.workspace.row);
        let result = lease
            .append(row, std::slice::from_ref(&projected), &mut joined)
            .and_then(|()| emit.row(lease, &joined));
        lease.workspace.row = joined;
        result
    }
}
