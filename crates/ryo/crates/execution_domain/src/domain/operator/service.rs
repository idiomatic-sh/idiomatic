//! Pure operator plan workspace and terminal access.
use crate::domain::{
    operating::errors::OperatingError, operation_schema::types::InputShape,
    operator::contract::types::OperatorContract,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_operators::types::OperationKind;

/// Read-only contract preparation. This surface cannot evaluate an operation.
pub fn prepare_operator(
    kind: OperationKind,
    operation: &Operation,
    inputs: &[InputShape<'_>],
    bindings: &[ParameterBinding],
) -> Result<OperatorContract, OperatingError> {
    super::contract::registration::registered(kind).prepare(operation, inputs, bindings)
}
