//! The project row semantics; dependencies and publication are engine-owned.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_dedup::service::first_time;
use crate::domain::table_model::types::Addition;
use crate::domain::table_model::types::ParameterBinding;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plans::types::{Operation, OperationKind};
pub(crate) struct ProjectItem;
impl OperatorsItem for ProjectItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputSchema<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::Project,
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
        let Operation::Project {
            columns, distinct, ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let mut projected = std::mem::take(&mut lease.workspace.other);
        let result = (|| {
            projected.clear();
            lease.project(row, columns, &mut projected)?;
            if *distinct == RepeatedRows::Dropped {
                let state = lease
                    .workspace
                    .distinct
                    .as_mut()
                    .ok_or(OperatingError::WrongBinding)?;
                if first_time(state.rows, state.seen, &projected) != Addition::Added {
                    return Ok(());
                }
            }
            emit.row(lease, &projected)
        })();
        lease.workspace.other = projected;
        result
    }
}
