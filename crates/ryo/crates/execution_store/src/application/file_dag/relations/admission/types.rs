//! The identity an admitted borrow was bound to.
use crate::application::file_dag::derived::population::service::PopulationId;
use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;

/// The population, source relation and generation an input was admitted under.
#[derive(Clone)]
pub(crate) struct AdmissionOrigin {
    pub population: PopulationId,
    pub relation: Option<Relation>,
    pub generation:
        Arc<crate::application::file_dag::relations::generation::service::GenerationLease>,
}
