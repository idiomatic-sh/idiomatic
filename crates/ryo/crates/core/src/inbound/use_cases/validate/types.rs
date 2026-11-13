//! The types of the `validate` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::hosted::types::HostedRelation;
use ryo_values::domain::query_tokens::types::QueryBundle;

/// The RyoQL text to check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidateRequest {
    /// Typed named producer inputs available to this query.
    pub observations: Vec<ryo_values::domain::observations::types::BoundObservationInput>,
    /// The query.
    pub text: QueryBundle,
    /// The relations the host declares, which the query may name: none when the caller declares none.
    pub hosted: Vec<HostedRelation>,
}
