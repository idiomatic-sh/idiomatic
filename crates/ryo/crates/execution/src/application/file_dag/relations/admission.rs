//! A demand-scoped immutable input and the one admission decision that belongs to it.
use super::{BorrowedRows, OperatingError, PhysicalShape, Relation, RelationOwner};
use crate::application::file_dag::derived::owner::PopulationId;
use std::sync::Arc;

/// Only the representation owner can bind population identity to borrowed input.
/// No provider reference or proof is retained beyond the immutable demand scope.
pub(in crate::application::file_dag) struct AdmittedInput<'a> {
    pub(super) rows: BorrowedRows<'a>,
    pub(super) shape: PhysicalShape,
    pub(super) backing_population: PopulationId,
    pub(super) backing_relation: Option<Relation>,
    pub(super) canonical_backing: bool,
    pub(super) origin: (PopulationId, Option<Relation>, Arc<()>),
}
impl<'a> AdmittedInput<'a> {
    /// Physical source identity bound once with this immutable, generation-checked input.
    /// Selected projections keep their own population for ordinary views while
    /// canonical graph columns can share the source backing identity.
    pub(in crate::application::file_dag) fn backing_population(
        &self,
        owner: &RelationOwner,
    ) -> Result<PopulationId, OperatingError> {
        self.identity(owner)?;
        Ok(self.backing_population)
    }
    pub(in crate::application::file_dag) fn graph_relation(
        &self,
        owner: &RelationOwner,
    ) -> Result<Option<Relation>, OperatingError> {
        self.identity(owner)?;
        Ok(self.backing_relation)
    }
    pub(in crate::application::file_dag) fn canonical_backing(&self) -> bool {
        self.canonical_backing
    }
    pub(in crate::application::file_dag) fn physical_shape(&self) -> PhysicalShape {
        self.shape
    }
    pub(in crate::application::file_dag) fn generation_guard(&self) -> Arc<()> {
        Arc::clone(&self.origin.2)
    }
    pub(in crate::application::file_dag) fn rows(&self) -> BorrowedRows<'a> {
        self.rows
    }
    pub(in crate::application::file_dag) fn identity(
        &self,
        owner: &RelationOwner,
    ) -> Result<(PopulationId, Option<Relation>), OperatingError> {
        let (population, relation, generation) = &self.origin;
        if !Arc::ptr_eq(generation, &owner.generation.borrow()) {
            return Err(OperatingError::NotRead);
        }
        if self.shape != PhysicalShape::Complete {
            return Err(OperatingError::ShapeMismatch);
        }
        Ok((*population, *relation))
    }
}
