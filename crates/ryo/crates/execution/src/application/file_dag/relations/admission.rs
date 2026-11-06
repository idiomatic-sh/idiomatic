//! A demand-scoped immutable input and the one admission decision that belongs to it.
use super::{BorrowedRows, OperatingError, PhysicalShape, Relation, RelationBody, RelationOwner};
use crate::application::file_dag::derived::population::types::PopulationId;
use crate::domain::table_model::collections::representations::publication::PublishedRelation;
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
impl<'a> AdmittedInput<'a> {
    pub(in crate::application::file_dag) fn require_publication(
        &self,
        owner: &RelationOwner,
        publication: &dyn PublishedRelation,
    ) -> Result<(), OperatingError> {
        let (population, _) = self.identity(owner)?;
        let actual = owner.private(publication)?;
        if self.publication_population != PopulationId(actual.population)
            || population != owner.result_population(actual)
            || !Arc::ptr_eq(&self.origin.2, &actual.generation)
        {
            return Err(OperatingError::NotRead);
        }
        Ok(())
    }

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

impl RelationOwner {
    pub(in crate::application::file_dag) fn admit<'a>(
        &self,
        result: &dyn PublishedRelation,
        input: BorrowedRows<'a>,
    ) -> Result<AdmittedInput<'a>, OperatingError> {
        self.inspect_available(result, input)?
    }

    pub(in crate::application::file_dag) fn inspect_available<'a>(
        &self,
        result: &dyn PublishedRelation,
        input: BorrowedRows<'a>,
    ) -> Result<Result<AdmittedInput<'a>, OperatingError>, OperatingError> {
        let result = self.private(result)?;
        if !self.matches_current(result, input) {
            return Err(OperatingError::NotRead);
        }
        // Identity is an availability refusal. Geometry is computed on the
        // first supplied borrow but can be consumed after static preflight.
        Ok(self
            .inspect_current(result, input)
            .map(|shape| AdmittedInput {
                publication_population: PopulationId(result.population),
                rows: input,
                shape,
                backing_population: self.backing_population(result),
                backing_relation: Self::backing_relation(result),
                canonical_backing: Self::backing_relation(result)
                    .is_some_and(|relation| self.sources.borrow().contains_key(&relation)),
                origin: (
                    self.result_population(result),
                    match &result.body {
                        RelationBody::Source(source) => Some(source.relation),
                        _ => None,
                    },
                    Arc::clone(&result.generation),
                ),
            }))
    }
}
