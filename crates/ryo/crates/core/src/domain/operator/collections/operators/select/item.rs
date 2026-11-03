//! Select semantics over one engine-scheduled row and its bound predicate program.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputSchema;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::{
    service::cell,
    types::{ParameterBinding, Verdict},
};
use ryo_values::domain::plans::types::{Operation, OperationKind};
pub(crate) struct SelectItem;
impl OperatorsItem for SelectItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[InputSchema<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::Select,
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
        let Operation::Select { kept, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let PreparedInputs::Row { row, .. } = inputs else {
            return Err(OperatingError::WrongOperation);
        };
        if lease.holds(row)? == Verdict::Fails {
            return Ok(());
        }
        if kept.is_empty() {
            return sink.row(lease, row);
        }
        let mut projected = std::mem::take(&mut lease.workspace.other);
        let outcome = (|| {
            projected.clear();
            for column in kept {
                projected.push(cell(row, *column)?);
            }
            sink.row(lease, &projected)
        })();
        lease.workspace.other = projected;
        outcome
    }
}
