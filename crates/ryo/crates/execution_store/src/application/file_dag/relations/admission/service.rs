//! A demand-scoped immutable input and the one admission decision that belongs to it.
use crate::application::file_dag::derived::population::service::PopulationId;
use crate::application::file_dag::relations::admission::types::AdmissionOrigin;
use ryo_execution_domain::domain::table_model::types::PhysicalShape;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_relations::domain::relations::types::Relation;

/// Only the representation owner can bind population identity to borrowed input.
/// No provider reference or proof is retained beyond the immutable demand scope.
#[derive(Clone)]
pub struct BorrowedInput<'a> {
    rows: RepresentationInput<'a>,
    shape: PhysicalShape,
    backing_population: PopulationId,
    backing_relation: Option<Relation>,
    canonical_backing: crate::application::file_dag::relations::types::BackingForm,
    origin: AdmissionOrigin,
    publication_population: PopulationId,
}

mod issuance;

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
