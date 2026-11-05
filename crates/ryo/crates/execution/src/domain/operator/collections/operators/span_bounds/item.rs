//! Project checked byte endpoints of a typed span without changing row order or bag.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::types::ParameterBinding;
use ryo_values::domain::plans::types::{Operation, OperationKind};
pub(crate) struct SpanBoundsItem;
impl OperatorsItem for SpanBoundsItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
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
        let projected = lease.span_bounds(row, *span)?;
        let mut joined = std::mem::take(&mut lease.workspace.row);
        let result = lease
            .append(row, &projected, &mut joined)
            .and_then(|()| emit.row(lease, &joined));
        lease.workspace.row = joined;
        result
    }
}
