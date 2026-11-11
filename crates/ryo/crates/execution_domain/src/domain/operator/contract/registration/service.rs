//! The `registration` service: free functions named by verb over the domain's types, with no IO and no ports.

use super::types::RegisteredOperator;
use ryo_values::domain::query_operators::types::OperationKind;

pub(in crate::domain::operator) fn prepare(
    registered: &RegisteredOperator,
    operation: &ryo_values::domain::plans::types::Operation,
    inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
    scope: ryo_values::domain::plan_parts::types::StepScope,
) -> Result<super::super::types::OperatorContract, crate::domain::operating::errors::OperatingError>
{
    if ryo_values::domain::plans::service::kind_of(operation) != registered.kind {
        return Err(crate::domain::operating::errors::OperatingError::WrongOperation);
    }
    super::super::service::prepare_for(registered.kind, operation, inputs, bindings, scope)
}

/// Evaluates an operation through the operators' one meaning of its kind.
///
/// # Errors
///
/// [`OperatingError::WrongOperation`](crate::domain::operating::errors::OperatingError::WrongOperation) when the
/// operation is not of this operator's kind, then the first error the evaluation or a downstream row returns.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::registration::service::evaluate;
/// let _ = evaluate;
/// ```
pub fn evaluate(
    registered: &RegisteredOperator,
    operation: &ryo_values::domain::plans::types::Operation,
    inputs: super::super::primitive::types::PreparedInputs<'_>,
    lease: &mut super::super::types::PrimitiveLease<'_>,
    output: &mut dyn super::super::types::RowSink,
) -> Result<(), crate::domain::operating::errors::OperatingError> {
    if ryo_values::domain::plans::service::kind_of(operation) != registered.kind {
        return Err(crate::domain::operating::errors::OperatingError::WrongOperation);
    }
    super::super::driver::service::evaluate(operation, inputs, lease, output)
}

/// The registered operator of a kind.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::registration::service::registered;
/// let _ = registered;
/// ```
pub fn registered(kind: OperationKind) -> RegisteredOperator {
    RegisteredOperator { kind }
}
