//! The intersection row semantics; dependencies and publication are engine-owned.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_operators::inbound::use_cases::operators::interface::SetKind;
use ryo_values::domain::plans::types::{Operation, OperationKind};
pub(crate) struct IntersectionItem;
impl OperatorsItem for IntersectionItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::Intersection,
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
        if !matches!(operation, Operation::Intersection { .. }) {
            return Err(OperatingError::WrongOperation);
        }
        crate::domain::operator::contract::set_rows::run(
            SetKind::Intersection,
            row,
            &[],
            lease,
            emit,
        )
    }
}
