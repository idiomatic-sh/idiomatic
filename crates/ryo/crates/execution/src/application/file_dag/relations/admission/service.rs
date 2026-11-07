//! A demand-scoped immutable input and the one admission decision that belongs to it.
use crate::application::file_dag::derived::population::types::PopulationId;
use crate::application::file_dag::relations::admission::types::AdmissionOrigin;
use crate::domain::{
    table_model::collections::representations::types::PhysicalShape,
};
use crate::domain::table_model::types::RepresentationInput as BorrowedRows;
use ryo_relations::domain::relations::types::Relation;

/// Only the representation owner can bind population identity to borrowed input.
/// No provider reference or proof is retained beyond the immutable demand scope.
#[derive(Clone)]
pub(in crate::application::file_dag) struct BorrowedInput<'a> {
    rows: BorrowedRows<'a>,
    shape: PhysicalShape,
    backing_population: PopulationId,
    backing_relation: Option<Relation>,
    canonical_backing: bool,
    origin: AdmissionOrigin,
    publication_population: PopulationId,
}

mod issuance;
