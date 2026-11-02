//! The intersection row semantics; dependencies and publication are engine-owned.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_dedup::service::first_time;
use crate::domain::table_model::types::Addition;
use crate::domain::table_model::types::ParameterBinding;
use ryo_values::domain::plans::types::{Operation, OperationKind};
pub(crate) struct IntersectionItem;
impl OperatorsItem for IntersectionItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputSchema<'_>],
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
        let index = lease
            .views
            .indices
            .first()
            .ok_or(OperatingError::WrongBinding)?;
        if !index.contains(row) {
            return Ok(());
        }
        let state = lease
            .workspace
            .distinct
            .as_mut()
            .ok_or(OperatingError::WrongBinding)?;
        if first_time(state.rows, state.seen, row) == Addition::Added {
            emit.row(lease, row)?;
        }
        Ok(())
    }
}
