//! The antijoin row semantics; dependencies and publication are engine-owned.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::service::cell;
use crate::domain::table_model::types::ParameterBinding;
use ryo_values::domain::plans::types::{Operation, OperationKind};
pub(crate) struct AntijoinItem;
impl OperatorsItem for AntijoinItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputSchema<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::Antijoin,
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
        let Operation::Antijoin { left_on, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let index = lease
            .views
            .indices
            .first()
            .ok_or(OperatingError::WrongBinding)?;
        // An empty right population keeps every left row without evaluating its keys.
        if index.rows.count() == 0 {
            return emit.row(lease, row);
        }
        let mut key = std::mem::take(lease.workspace.row);
        let result = (|| {
            key.clear();
            for column in left_on {
                key.push(cell(row, *column)?);
            }
            if !index.contains(&key) {
                emit.row(lease, row)?;
            }
            Ok(())
        })();
        *lease.workspace.row = key;
        result
    }
}
