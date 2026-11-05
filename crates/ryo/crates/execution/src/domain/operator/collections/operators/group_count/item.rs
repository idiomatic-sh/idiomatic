//! Emit semantic aggregates from the contributing population and engine-prepared grouping.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::{service::cell, types::ParameterBinding};
use ryo_relations::domain::relations::types::{Integer, Value};
use ryo_values::domain::plans::types::{Operation, OperationKind};
pub(crate) struct GroupCountItem;
impl OperatorsItem for GroupCountItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::GroupCount,
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
        sink: &mut dyn RowSink,
    ) -> Result<(), OperatingError> {
        let Operation::GroupCount { keys, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let PreparedInputs::Views(inputs) = inputs else {
            return Err(OperatingError::WrongOperation);
        };
        let source = inputs.first().ok_or(OperatingError::NotRun)?;
        let groups = lease
            .views
            .resources
            .groups()
            .ok_or(OperatingError::NotRun)?;
        let mut row = std::mem::take(&mut lease.workspace.row);
        let mut answer = std::mem::take(&mut lease.workspace.other);
        let outcome = (|| {
            for group in groups {
                let first = source
                    .read(group.first, &mut row)
                    .ok_or(OperatingError::NotRun)?;
                answer.clear();
                for key in keys {
                    answer.push(cell(first, *key)?);
                }

                answer.push(Value::Number(Integer(
                    i64::try_from(group.counted.0).unwrap_or(i64::MAX),
                )));
                sink.row(lease, &answer)?;
            }
            Ok(())
        })();
        lease.workspace.row = row;
        lease.workspace.other = answer;
        outcome
    }
}
