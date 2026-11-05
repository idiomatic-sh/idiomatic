//! Pure operator plan workspace and terminal access.
use crate::domain::{
    operating::errors::OperatingError, operation_schema::types::InputShape,
    operator::contract::types::OperatorContract,
};
use ryo_values::domain::plans::types::{Operation, OperationKind};

/// Read-only contract preparation. This surface cannot evaluate an operation.
pub(crate) struct PreparationRoute(OperationKind);
impl PreparationRoute {
    pub(crate) fn prepare(
        &self,
        operation: &Operation,
        inputs: &[InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        super::contract::registration::registered(self.0).prepare(operation, inputs, bindings)
    }
}
pub(crate) fn registered(kind: OperationKind) -> PreparationRoute {
    PreparationRoute(kind)
}

use ryo_values::domain::query_bindings::types::ParameterBinding;
