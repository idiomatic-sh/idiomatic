//! Pure operator plan workspace and terminal access.
use crate::domain::{
    operating::errors::OperatingError,
    operation_schema::types::InputShape,
    operator::contract::types::OperatorContract,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_operators::types::OperationKind;

/// Read-only contract preparation. This surface cannot evaluate an operation.
///
/// # Errors
///
/// The [`OperatingError`] the operator registered for `kind` refuses the operation, its input shapes or its bindings with.
pub fn prepare_operator(
    kind: OperationKind,
    operation: &Operation,
    inputs: &[InputShape<'_>],
    bindings: &[ParameterBinding],
    scope: ryo_values::domain::plan_parts::types::StepScope,
) -> Result<OperatorContract, OperatingError> {
    crate::domain::operator::contract::registration::service::prepare(
        &super::contract::registration::service::registered(kind),
        operation,
        inputs,
        bindings,
        scope,
    )
}
