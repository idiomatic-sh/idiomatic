//! Host materialization and symbol provenance belong to the request owner.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::types::ParameterBinding;
use ryo_values::domain::plans::types::{Operation, OperationKind};
pub(crate) struct HostedItem;
impl OperatorsItem for HostedItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::Hosted,
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
        if !matches!(operation, Operation::Hosted { .. }) {
            return Err(OperatingError::WrongOperation);
        }
        let PreparedInputs::RequestSource(source) = inputs else {
            return Err(OperatingError::WrongOperation);
        };
        let mut row = std::mem::take(&mut lease.workspace.row);
        let result = (|| {
            for at in 0..source.count() {
                if let Some(each) =
                    source.read(ryo_values::domain::rows::types::RowIndex(at), &mut row)
                {
                    sink.row(lease, each)?;
                }
            }
            Ok(())
        })();
        lease.workspace.row = row;
        result
    }
}
