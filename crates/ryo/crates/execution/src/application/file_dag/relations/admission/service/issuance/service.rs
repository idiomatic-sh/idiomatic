//! Owner-dependent admission logic lives below its private data representation.
use super::super::*;
use crate::application::file_dag::relations::admission::types::AdmissionOrigin;
use crate::application::file_dag::relations::publication::types::*;
use crate::application::file_dag::relations::types::Sameness;
use crate::application::file_dag::relations::*;
use ryo_execution_domain::domain::table_model::types::PublishedRelation;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
impl<'a> BorrowedInput<'a> {
    pub(crate) fn require_publication(
        &self,
        owner: &RelationOwner,
        publication: &dyn PublishedRelation,
    ) -> Result<(), OperatingError> {
        let population = self.identity(owner)?.population;
        let actual = owner.private(publication)?;
        if self.publication_population != actual.population
            || population != owner.result_population(actual)
            || !Arc::ptr_eq(&self.origin.generation, &actual.generation)
        {
            return Err(OperatingError::NotRead);
        }
        Ok(())
    }

    /// Physical source identity bound once with this immutable, generation-checked input.
    /// Selected projections keep their own population for ordinary views while
    /// canonical graph columns can share the source backing identity.
    pub(crate) fn backing_population(
        &self,
        owner: &RelationOwner,
    ) -> Result<PopulationId, OperatingError> {
        self.identity(owner)?;
        Ok(self.backing_population)
    }
    pub(crate) fn graph_relation(
        &self,
        owner: &RelationOwner,
    ) -> Result<Option<Relation>, OperatingError> {
        self.identity(owner)?;
        Ok(self.backing_relation)
    }
    pub(crate) fn canonical_backing(
        &self,
    ) -> crate::application::file_dag::relations::types::BackingForm {
        self.canonical_backing
    }
    pub(crate) fn physical_shape(&self) -> PhysicalShape {
        self.shape
    }
    pub(crate) fn generation_guard(&self) -> Arc<()> {
        Arc::clone(&self.origin.generation)
    }
    pub(crate) fn rows(&self) -> RepresentationInput<'a> {
        self.rows
    }
    pub(crate) fn identity(
        &self,
        owner: &RelationOwner,
    ) -> Result<crate::application::file_dag::relations::admission::types::AdmissionIdentity, OperatingError> {
        if !Arc::ptr_eq(&self.origin.generation, &owner.generation.borrow()) {
            return Err(OperatingError::NotRead);
        }
        if self.shape != PhysicalShape::Complete {
            return Err(OperatingError::ShapeMismatch);
        }
        Ok(crate::application::file_dag::relations::admission::types::AdmissionIdentity {
            population: self.origin.population,
            relation: self.origin.relation,
        })
    }
}

impl RelationOwner {
    pub(crate) fn admit<'a>(
        &self,
        result: &dyn PublishedRelation,
        input: RepresentationInput<'a>,
    ) -> Result<BorrowedInput<'a>, OperatingError> {
        self.inspect_available(result, input)?
    }

    pub(crate) fn inspect_available<'a>(
        &self,
        result: &dyn PublishedRelation,
        input: RepresentationInput<'a>,
    ) -> Result<Result<BorrowedInput<'a>, OperatingError>, OperatingError> {
        let result = self.private(result)?;
        if self.matches_current(result, input) == Sameness::Different {
            return Err(OperatingError::NotRead);
        }
        // Identity is an availability refusal. Geometry is computed on the
        // first supplied borrow but can be consumed after static preflight.
        Ok(self
            .inspect_current(result, input)
            .map(|shape| BorrowedInput {
                publication_population: result.population,
                rows: input,
                shape,
                backing_population: self.backing_population(result),
                backing_relation: Self::backing_relation(result),
                canonical_backing: if Self::backing_relation(result)
                    .is_some_and(|relation| self.sources.borrow().contains_key(&relation))
                {
                    crate::application::file_dag::relations::types::BackingForm::Canonical
                } else {
                    crate::application::file_dag::relations::types::BackingForm::Foreign
                },
                origin: AdmissionOrigin {
                    population: self.result_population(result),
                    relation: match &result.body {
                        RelationBody::Source(source) => Some(source.relation),
                        _ => None,
                    },
                    generation: Arc::clone(&result.generation),
                },
            }))
    }
}
