//! The identity an admitted borrow was bound to.
use crate::application::file_dag::derived::population::types::PopulationId;
use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;

/// The population, source relation and generation an input was admitted under.
#[derive(Clone)]
pub(crate) struct AdmissionOrigin {
    pub population: PopulationId,
    pub relation: Option<Relation>,
    pub generation: Arc<crate::application::file_dag::relations::generation::service::GenerationLease>,
}

/// The population and source relation an input was admitted under, without its generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionIdentity {
    pub(crate) population: PopulationId,
    pub(crate) relation: Option<Relation>,
}
impl AdmissionIdentity {
    /// The population the input was admitted under.
    pub fn population(&self) -> PopulationId {
        self.population
    }
    /// The source relation the input was admitted from, when it came from one.
    pub fn relation(&self) -> Option<Relation> {
        self.relation
    }
}
