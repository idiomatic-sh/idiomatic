//! The admitted input: identity, rows, coverage and generation validation.
use super::super::{
    AdmittedInput, AdmittedWalkSource, Agreement, ObservationReadiness, Partition, PartitionSupply,
    PopulationProof, ProducerRoot, Readiness,
};
use crate::application::file_dag::derived::population::types::PopulationId;
use crate::application::file_dag::relations::{RelationOwner, handle::service::PublishedInput};
use crate::domain::{executable_program::types::InputSource, operating::errors::OperatingError};
use std::{rc::Rc, sync::Arc};

impl AdmittedInput {
    pub(in crate::application::file_dag::program) fn population(
        &self,
        owner: &Rc<RelationOwner>,
    ) -> Result<
        crate::application::file_dag::derived::population::types::PopulationId,
        OperatingError,
    > {
        self.publication(owner)?;
        Ok(self.proof.population)
    }
    pub(in crate::application::file_dag::program) fn has_root(
        &self,
        root: ProducerRoot,
    ) -> Agreement {
        if self.root == root {
            Agreement::Same
        } else {
            Agreement::Different
        }
    }
    /// Every admitted consumer uses the same physical-layout contract. A readable
    /// malformed provider is normalized once by its owner; complete native and
    /// compact populations remain borrowed in their original representation.
    pub(in crate::application::file_dag::program) fn rows<'a, 'facts>(
        &'a self,
        owner: &'a Rc<RelationOwner>,
        read: &dyn Fn(
            ryo_relations::domain::relations::types::Relation,
        )
            -> Result<crate::domain::table_model::types::Rows<'facts>, OperatingError>,
    ) -> Result<crate::domain::table_model::types::Rows<'a>, OperatingError>
    where
        'facts: 'a,
    {
        let publication = self.publication(owner)?;
        let rows = owner.resolve(publication.as_ref(), read)?;
        owner.normalize(publication, rows)
    }
    pub(in crate::application::file_dag::program) fn observation_ready(
        &self,
    ) -> Result<ObservationReadiness, OperatingError> {
        match &self.readiness {
            Readiness::Complete => Ok(ObservationReadiness::Ready),
            Readiness::Partial { .. } | Readiness::Inapplicable { .. } => {
                Ok(ObservationReadiness::Waiting)
            }
        }
    }

    pub(in crate::application::file_dag::program) fn coverage(
        &self,
    ) -> Result<crate::domain::completion::types::CoverageState, OperatingError> {
        match &self.readiness {
            Readiness::Complete => Ok(crate::domain::completion::types::CoverageState::Complete),
            Readiness::Partial { files } => {
                Ok(crate::domain::completion::types::CoverageState::Partial {
                    files: files.clone(),
                })
            }
            Readiness::Inapplicable { files } => Ok(
                crate::domain::completion::types::CoverageState::Inapplicable {
                    files: files.clone(),
                },
            ),
        }
    }
    pub(in crate::application::file_dag::program) fn with_coverage(
        mut self,
        coverage: crate::domain::completion::types::CoverageState,
    ) -> Self {
        use crate::domain::completion::types::CoverageState;
        match coverage {
            CoverageState::Complete => {}
            CoverageState::Partial { mut files } => match &mut self.readiness {
                Readiness::Complete => self.readiness = Readiness::Partial { files },
                Readiness::Partial { files: existing } => {
                    for file in files.drain(..) {
                        if !existing.contains(&file) {
                            existing.push(file);
                        }
                    }
                }
                Readiness::Inapplicable { files: existing } => {
                    files.extend(existing.iter().copied().map(Some));
                    files.sort_unstable();
                    files.dedup();
                    self.readiness = Readiness::Partial { files };
                }
            },
            CoverageState::Inapplicable { files } => match &mut self.readiness {
                Readiness::Complete => self.readiness = Readiness::Inapplicable { files },
                Readiness::Partial { files: existing } => {
                    for file in files {
                        if !existing.contains(&Some(file)) {
                            existing.push(Some(file));
                        }
                    }
                }
                Readiness::Inapplicable { files: existing } => {
                    for file in files {
                        if !existing.contains(&file) {
                            existing.push(file);
                        }
                    }
                }
            },
        }
        self
    }

    pub(in crate::application::file_dag::program) fn validate_for(
        &self,
        owner: &Rc<RelationOwner>,
        generation: &Arc<()>,
        required: &Partition,
        expected: ProducerRoot,
    ) -> Result<(), OperatingError> {
        if owner.matches_generation(generation) == crate::application::file_dag::relations::types::Sameness::Different
            || !Rc::ptr_eq(owner, &self.proof.owner)
            || !Arc::ptr_eq(&self.proof.generation, generation)
            || self.root != expected
            || self.partition.supplies(required) == PartitionSupply::Lacks
        {
            return Err(OperatingError::NotRead);
        }
        self.publication(owner)?;
        Ok(())
    }
    pub(in crate::application::file_dag::program) fn validate_publication(
        &self,
    ) -> Result<(), OperatingError> {
        self.publication(&self.proof.owner).map(|_| ())
    }
    pub(in crate::application::file_dag::program) fn walk_source<'call>(
        &'call self,
        relation: ryo_relations::domain::relations::types::Relation,
        rows: &'call crate::domain::table_model::types::Rows<'call>,
        generation: &Arc<()>,
    ) -> Result<Option<AdmittedWalkSource<'call>>, OperatingError> {
        use crate::domain::{
            operator::contract::views::BorrowedRows,
            table_model::{service::kinds_of, types::Rows},
        };
        if self.root != ProducerRoot::Input(InputSource::Relation(relation)) {
            return Err(OperatingError::WrongOperation);
        }
        if !Arc::ptr_eq(&self.proof.generation, generation) {
            return Err(OperatingError::NotRead);
        }
        let publication = self.publication(&self.proof.owner)?;
        let admitted = self
            .proof
            .owner
            .admit(publication.as_ref(), BorrowedRows::from(rows))?;
        let (population, actual_relation) = admitted.identity(&self.proof.owner)?;
        if population != self.proof.population || actual_relation != Some(relation) {
            return Err(OperatingError::NotRead);
        }
        if kinds_of(rows) != ryo_relations::domain::relations::service::kinds(relation)
            || matches!(rows,Rows::Tree(tree) if crate::domain::table_cells::service::native_form(relation) != Some(tree.form))
        {
            return Ok(None);
        }
        Ok(Some(AdmittedWalkSource {
            source: self,
            rows,
            relation,
            population,
        }))
    }
    /// Only a real generation-checked admission can issue this required input.
    pub(in crate::application::file_dag::program) fn from_publication(
        owner: Rc<RelationOwner>,
        publication: PublishedInput,
        admitted: &crate::application::file_dag::relations::admission::service::BorrowedInput<'_>,
        root: ProducerRoot,
        partition: Partition,
        readiness: Readiness,
    ) -> Result<Self, OperatingError> {
        admitted.require_publication(owner.as_ref(), publication.as_ref())?;
        let (population, _) = admitted.identity(&owner)?;
        let generation = admitted.generation_guard();

        Ok(Self {
            proof: PopulationProof {
                owner,
                population,
                generation,
            },
            root,
            partition,
            readiness,
            publication,
        })
    }
    pub(in crate::application::file_dag::program) fn publication(
        &self,
        owner: &Rc<RelationOwner>,
    ) -> Result<&PublishedInput, OperatingError> {
        if !Rc::ptr_eq(owner, &self.proof.owner)
            || owner.matches_generation(&self.proof.generation) == crate::application::file_dag::relations::types::Sameness::Different
        {
            return Err(OperatingError::NotRead);
        }
        match &self.readiness {
            Readiness::Complete | Readiness::Partial { .. } | Readiness::Inapplicable { .. } => {
                Ok(&self.publication)
            }
        }
    }
}
