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

/// The registered operator of a kind.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::registration::service::registered;
/// use ryo_values::domain::query_operators::types::OperationKind;
///
/// let select = registered(OperationKind::Select);
/// assert!(format!("{select:?}").contains("Select"));
/// ```
pub fn registered(kind: OperationKind) -> RegisteredOperator {
    RegisteredOperator { kind }
}
