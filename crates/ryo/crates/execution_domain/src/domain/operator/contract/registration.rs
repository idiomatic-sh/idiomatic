//! The mandatory exhaustive executable registry. No optional or residual path exists: every kind is prepared
//! by its declared contract and evaluated by the operators' one meaning of it.
use ryo_values::domain::query_operators::types::OperationKind;
/// Only this registry constructs a callable operator selected by the exhaustive kind.
#[derive(Debug, Clone, Copy)]
pub struct RegisteredOperator {
    kind: OperationKind,
}
impl RegisteredOperator {
    pub(in crate::domain::operator) fn prepare(
        &self,
        operation: &ryo_values::domain::plans::types::Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
        bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
        scope: ryo_values::domain::plan_parts::types::StepScope,
    ) -> Result<super::types::OperatorContract, crate::domain::operating::errors::OperatingError>
    {
        if ryo_values::domain::plans::service::kind_of(operation) != self.kind {
            return Err(crate::domain::operating::errors::OperatingError::WrongOperation);
        }
        super::service::prepare_for(self.kind, operation, inputs, bindings, scope)
    }
    /// Evaluates an operation through the operators' one meaning of its kind.
    ///
    /// # Errors
    ///
    /// [`OperatingError::WrongOperation`](crate::domain::operating::errors::OperatingError::WrongOperation) when the
    /// operation is not of this operator's kind, then the first error the evaluation or a downstream row returns.
    pub fn evaluate(
        &self,
        operation: &ryo_values::domain::plans::types::Operation,
        inputs: super::primitive::PreparedInputs<'_>,
        lease: &mut super::types::PrimitiveLease<'_>,
        output: &mut dyn super::types::RowSink,
    ) -> Result<(), crate::domain::operating::errors::OperatingError> {
        if ryo_values::domain::plans::service::kind_of(operation) != self.kind {
            return Err(crate::domain::operating::errors::OperatingError::WrongOperation);
        }
        super::driver::evaluate(operation, inputs, lease, output)
    }
}
/// The registered operator of a kind.
pub fn registered(kind: OperationKind) -> RegisteredOperator {
    RegisteredOperator { kind }
}
