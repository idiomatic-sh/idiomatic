//! The types of the `validate` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::hosted::types::HostedRelation;
use ryo_values::domain::plan_parts::types::QueryText;

/// The RyoQL text to check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidateRequest {
    /// The query.
    pub text: QueryText,
    /// The relations the host declares, which the query may name: none when the caller declares none.
    pub hosted: Vec<HostedRelation>,
}
