//! What an extension lowers to.

use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// The values an extension appends to each row and what their parameters are bound to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ExtendParts {
    /// One operand for each new column: a written number or text, or a parameter.
    pub(crate) values: Vec<Operand>,
    /// What the parameters are bound to.
    pub(crate) bindings: Vec<ParameterBinding>,
}
