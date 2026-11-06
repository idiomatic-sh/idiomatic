//! Select semantics over one engine-scheduled row and its bound predicate program.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::types::Verdict;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;
pub(crate) struct SelectItem;
impl OperatorsItem for SelectItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[InputShape<'_>],
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
        let holds = lease.holds(row)? != Verdict::Fails;
        let mut projected = std::mem::take(&mut lease.workspace.other);
        let answer = lease.semantics().operators(
            ryo_operators::inbound::use_cases::operators::interface::OperatorCall::Select {
                row,
                kept,
                holds,
                target: &mut projected,
            },
        );
        let outcome = match answer {
            Ok(
                ryo_operators::inbound::use_cases::operators::interface::OperatorAnswer::Selected(
                    false,
                ),
            ) => Ok(()),
            Ok(
                ryo_operators::inbound::use_cases::operators::interface::OperatorAnswer::Selected(
                    true,
                ),
            ) => sink.row(lease, if kept.is_empty() { row } else { &projected }),
            Err(error) => Err(crate::domain::operator::contract::types::portable_fault(
                error,
            )),
            _ => Err(OperatingError::WrongOperation),
        };
        lease.workspace.other = projected;
        outcome
    }
}

use ryo_values::domain::query_bindings::types::ParameterBinding;
