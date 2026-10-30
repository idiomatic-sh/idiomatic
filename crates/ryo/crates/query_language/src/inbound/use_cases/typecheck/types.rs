//! The types of the `typecheck` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::plan_parts::types::QueryText;

/// The RyoQL text to check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypecheckRequest {
    /// The query.
    pub text: QueryText,
    /// The relations the host declares, which the query may name.
    pub hosted: Vec<ryo_values::domain::hosted::types::HostedRelation>,
}
