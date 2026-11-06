//! A demand-scoped immutable input and the one admission decision that belongs to it.
use crate::application::file_dag::derived::population::types::PopulationId;
use crate::domain::{
    operator::contract::views::BorrowedRows,
    table_model::collections::representations::types::PhysicalShape,
};
use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;

/// Only the representation owner can bind population identity to borrowed input.
/// No provider reference or proof is retained beyond the immutable demand scope.
#[derive(Clone)]
pub(in crate::application::file_dag) struct AdmittedInput<'a> {
    rows: BorrowedRows<'a>,
    shape: PhysicalShape,
    backing_population: PopulationId,
    backing_relation: Option<Relation>,
    canonical_backing: bool,
    origin: (PopulationId, Option<Relation>, Arc<()>),
    publication_population: PopulationId,
}

mod issuance;
