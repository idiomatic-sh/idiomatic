//! The identity an admitted borrow was bound to.
use crate::application::file_dag::derived::population::types::PopulationId;
use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;

/// The population, source relation and generation an input was admitted under.
#[derive(Clone)]
pub(in crate::application::file_dag) struct AdmissionOrigin {
    pub population: PopulationId,
    pub relation: Option<Relation>,
    pub generation: Arc<crate::application::file_dag::relations::generation::service::GenerationLease>,
}

/// The population and source relation an input was admitted under, without its generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) struct AdmissionIdentity {
    pub population: PopulationId,
    pub relation: Option<Relation>,
}
