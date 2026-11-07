//! The identity an admitted borrow was bound to.
use crate::application::file_dag::derived::population::types::PopulationId;
use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;

/// The population, source relation and generation an input was admitted under.
#[derive(Clone)]
pub(in crate::application::file_dag) struct AdmissionOrigin {
    pub population: PopulationId,
    pub relation: Option<Relation>,
    pub generation: Arc<()>,
}
