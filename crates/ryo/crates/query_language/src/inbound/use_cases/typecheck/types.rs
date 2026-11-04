//! The types of the `typecheck` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::program::types::QueryBundle;

/// The RyoQL text to check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypecheckRequest {
    /// Typed named producer inputs available to this query.
    pub observations: Vec<ryo_values::domain::observations::types::BoundObservationSchema>,
    /// The query.
    pub text: QueryBundle,
    /// The relations the host declares, which the query may name.
    pub hosted: Vec<ryo_values::domain::hosted::types::HostedRelation>,
}
