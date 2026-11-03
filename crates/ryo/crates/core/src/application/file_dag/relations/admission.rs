//! A demand-scoped immutable input and the one admission decision that belongs to it.
use super::{BorrowedRows, OperatingError, PhysicalShape, Relation, RelationOwner};
use crate::application::file_dag::derived::owner::PopulationId;
use std::sync::Arc;

/// Only the representation owner can bind population identity to borrowed input.
/// No provider reference or proof is retained beyond the immutable demand scope.
pub(in crate::application::file_dag) struct AdmittedInput<'a> {
    pub(super) rows: BorrowedRows<'a>,
    pub(super) shape: PhysicalShape,
    pub(super) origin: (PopulationId, Option<Relation>, Arc<()>),
}
impl<'a> AdmittedInput<'a> {
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
