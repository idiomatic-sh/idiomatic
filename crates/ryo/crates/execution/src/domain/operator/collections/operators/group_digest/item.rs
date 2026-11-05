//! Emit semantic aggregates from the contributing population and engine-prepared grouping.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use ryo_values::domain::plans::types::{Operation, OperationKind};
use ryo_values::domain::query_bindings::types::ParameterBinding;
pub(crate) struct GroupDigestItem;
impl OperatorsItem for GroupDigestItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::GroupDigest,
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
        let Operation::GroupDigest { keys, .. } = operation else {
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
                let semantic = lease.semantics().operators(
                    ryo_operators::inbound::use_cases::operators::interface::OperatorCall::Aggregate {
                        row: first, keys, count: group.counted.0, digest: Some(group.digest.0), target: &mut answer,
                    },
                );
                crate::domain::operator::contract::types::written_result(semantic, None)?;
                sink.row(lease, &answer)?;
            }
            Ok(())
        })();
        lease.workspace.row = row;
        lease.workspace.other = answer;
        outcome
    }
}
