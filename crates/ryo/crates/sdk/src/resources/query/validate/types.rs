//! The types of `query validate`: what is asked, and what a valid query uses.

use crate::domain::host::types::HostDeclaration;
use crate::domain::operator::types::OperatorName;
use crate::domain::query::types::RyoQlQuery;
use crate::domain::relation::types::RelationName;

/// How many steps a query compiles to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepCount(pub usize);

/// What a valid query uses.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ValidateReport {
    /// The relations it reads, each once.
    pub relations: Vec<RelationName>,
    /// The operations it uses, each once.
    pub operators: Vec<OperatorName>,
    /// How many steps it compiles to.
    pub steps: StepCount,
}

/// What `query validate` is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidateRequest {
    /// Typed named producer inputs available to this query.
    pub observations: Vec<crate::domain::observation::types::BoundObservationSchema>,
    /// The query to check.
    pub query: RyoQlQuery,
    /// The relations the host declares, which the query may name.
    pub hosted: Vec<HostDeclaration>,
}
