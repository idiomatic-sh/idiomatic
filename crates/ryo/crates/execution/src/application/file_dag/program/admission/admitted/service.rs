//! The admitted input: identity, rows, coverage and generation validation.
use super::super::{
    AdmittedInput, AdmittedWalkSource, Agreement, ObservationReadiness, Partition, PartitionSupply,
    PopulationProof, ProducerRoot, ReadProof, Readiness,
};
use crate::application::file_dag::derived::population::types::PopulationId;
use crate::application::file_dag::relations::{RelationOwner, handle::service::PublishedInput};
use crate::domain::{executable_program::types::InputSource, operating::errors::OperatingError};
use std::{rc::Rc, sync::Arc};

impl ReadProof {
    /// A whole physical read of `partition`.
    pub(in crate::application::file_dag::program) fn complete(partition: Partition) -> Self {
        Self {
            partition,
            readiness: Readiness::Complete,
        }
    }
    /// A read of `partition` whose coverage the completion domain computed.
    pub(in crate::application::file_dag::program) fn covered(
        partition: Partition,
        coverage: crate::domain::completion::types::CoverageState,
    ) -> Self {
        use crate::domain::completion::types::CoverageState;
        let readiness = match coverage {
            CoverageState::Complete => Readiness::Complete,
            CoverageState::Partial { files, lacks } => Readiness::Partial { files, lacks },
            CoverageState::Inapplicable { files } => Readiness::Inapplicable { files },
        };
        Self {
            partition,
            readiness,
        }
    }
    pub(in crate::application::file_dag::program) fn observation(&self) -> ObservationReadiness {
        match self.readiness {
            Readiness::Complete => ObservationReadiness::Ready,
            Readiness::Partial { .. } | Readiness::Inapplicable { .. } => {
                ObservationReadiness::Waiting
            }
        }
    }
}

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
            Readiness::Partial { files, lacks } => {
                Ok(crate::domain::completion::types::CoverageState::Partial {
                    files: files.clone(),
                    lacks: lacks.clone(),
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
            CoverageState::Partial { files, lacks } => {
                use crate::domain::completion::service::{lacks_of, partial_of_lacks};
                let mut all = lacks_of(&files, &lacks);
                match &self.readiness {
                    Readiness::Complete => {}
                    Readiness::Partial {
                        files: existing,
                        lacks: existing_lacks,
                    } => all.extend(lacks_of(existing, existing_lacks)),
                    Readiness::Inapplicable { files: existing } => {
                        all.extend(lacks_of(
                            &existing.iter().copied().map(Some).collect::<Vec<_>>(),
                            &[],
                        ));
                    }
                }
                if let CoverageState::Partial { files, lacks } = partial_of_lacks(all) {
                    self.readiness = Readiness::Partial { files, lacks };
                }
            }
            CoverageState::Inapplicable { files } => match &mut self.readiness {
                Readiness::Complete => self.readiness = Readiness::Inapplicable { files },
                Readiness::Partial {
                    files: existing,
                    lacks,
                } => {
                    // A file that is out of scope lacks any row of itself.
                    for file in files {
                        lacks.retain(|each| each.file != Some(file));
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
    pub(in crate::application::file_dag::program) fn walk_source<'call>(
        &'call self,
        relation: ryo_relations::domain::relations::types::Relation,
        rows: &'call crate::domain::table_model::types::Rows<'call>,
        generation: &Arc<()>,
    ) -> Result<Option<AdmittedWalkSource<'call>>, OperatingError> {
        use crate::domain::{
            table_model::{service::kinds_of, types::Rows},
        };
use crate::domain::table_model::types::RepresentationInput;
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
            .admit(publication.as_ref(), RepresentationInput::from(rows))?;
        let crate::application::file_dag::relations::admission::types::AdmissionIdentity {
            population,
            relation: actual_relation,
        } = admitted.identity(&self.proof.owner)?;
        // A project-stage input is the decoded frontier of the relation: computed rows carry no relation identity of their own, and
        // the root and the population proof already say which relation they are.
        if population != self.proof.population
            || actual_relation.is_some_and(|actual| actual != relation)
        {
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
        read: ReadProof,
    ) -> Result<Self, OperatingError> {
        admitted.require_publication(owner.as_ref(), publication.as_ref())?;
        let crate::application::file_dag::relations::admission::types::AdmissionIdentity { population, .. } = admitted.identity(&owner)?;
        let generation = admitted.generation_guard();

        Ok(Self {
            proof: PopulationProof {
                owner,
                population,
                generation,
            },
            root,
            partition: read.partition,
            readiness: read.readiness,
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
