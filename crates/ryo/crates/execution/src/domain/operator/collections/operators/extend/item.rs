//! Append values resolved by the engine at the declared input boundary.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::query_bindings::types::ParameterBinding;
pub(crate) struct ExtendItem;
impl OperatorsItem for ExtendItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::Extend,
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
        if !matches!(operation, Operation::Extend { .. }) {
            return Err(OperatingError::WrongOperation);
        }
        let appended = lease
            .views
            .resources
            .appended()
            .ok_or(OperatingError::WrongBinding)?;
        let mut joined = std::mem::take(&mut lease.workspace.row);
        let result = lease
            .append(row, appended, &mut joined)
            .and_then(|()| emit.row(lease, &joined));
        lease.workspace.row = joined;
        result
    }
}
