//! The admitted input: identity, rows, coverage and generation validation.
use super::super::{
    AdmittedBody, AdmittedInput, AdmittedWalkSource, Agreement, ObservationReadiness, Partition, PartitionSupply,
    PopulationProof, ProducerRoot, ReadProof, Readiness,
};
use ryo_execution_store::application::file_dag::relations::{RelationOwner, handle::service::PublishedInput};
use ryo_execution_domain::domain::{executable_program::types::InputSource, operating::errors::OperatingError};
use std::{rc::Rc, sync::Arc};

impl ReadProof {
    /// A whole physical read of `partition`.
    pub(crate) fn complete(partition: Partition) -> Self {
        Self {
            partition,
            readiness: Readiness::Complete,
        }
    }
    /// A read of `partition` whose coverage the completion domain computed.
    pub(crate) fn covered(
        partition: Partition,
        coverage: ryo_execution_domain::domain::completion::types::CoverageState,
    ) -> Self {
        use ryo_execution_domain::domain::completion::types::CoverageState;
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
    pub(crate) fn observation(&self) -> ObservationReadiness {
        match self.readiness {
            Readiness::Complete => ObservationReadiness::Ready,
            Readiness::Partial { .. } | Readiness::Inapplicable { .. } => {
                ObservationReadiness::Waiting
            }
        }
    }
}

impl AdmittedInput {
    pub(crate) fn population(
        &self,
        owner: &Rc<RelationOwner>,
    ) -> Result<
        ryo_execution_store::application::file_dag::derived::population::types::PopulationId,
        OperatingError,
    > {
        self.publication(owner)?;
        Ok(self.body.proof.population)
    }
    pub(crate) fn has_root(
        &self,
        root: ProducerRoot,
    ) -> Agreement {
        if self.body.root == root {
            Agreement::Same
        } else {
            Agreement::Different
        }
    }
    /// Every admitted consumer uses the same physical-layout contract. A readable
    /// malformed provider is normalized once by its owner; complete native and
    /// compact populations remain borrowed in their original representation.
    pub(crate) fn rows<'a, 'facts>(
        &'a self,
        owner: &'a Rc<RelationOwner>,
        read: &dyn Fn(
            ryo_relations::domain::relations::types::Relation,
        )
            -> Result<ryo_execution_domain::domain::table_model::types::Rows<'facts>, OperatingError>,
    ) -> Result<ryo_execution_domain::domain::table_model::types::Rows<'a>, OperatingError>
    where
        'facts: 'a,
    {
        let publication = self.publication(owner)?;
        let rows = owner.resolve(publication.as_ref(), read)?;
        owner.normalize(publication, rows)
    }
    pub(crate) fn observation_ready(
        &self,
    ) -> Result<ObservationReadiness, OperatingError> {
        match &self.body.readiness {
            Readiness::Complete => Ok(ObservationReadiness::Ready),
            Readiness::Partial { .. } | Readiness::Inapplicable { .. } => {
                Ok(ObservationReadiness::Waiting)
            }
        }
    }

    pub(crate) fn coverage(
        &self,
    ) -> Result<ryo_execution_domain::domain::completion::types::CoverageState, OperatingError> {
        match &self.body.readiness {
            Readiness::Complete => Ok(ryo_execution_domain::domain::completion::types::CoverageState::Complete),
            Readiness::Partial { files, lacks } => {
                Ok(ryo_execution_domain::domain::completion::types::CoverageState::Partial {
                    files: files.clone(),
                    lacks: lacks.clone(),
                })
            }
            Readiness::Inapplicable { files } => Ok(
                ryo_execution_domain::domain::completion::types::CoverageState::Inapplicable {
                    files: files.clone(),
                },
            ),
        }
    }
    pub(crate) fn with_coverage(
        mut self,
        coverage: ryo_execution_domain::domain::completion::types::CoverageState,
    ) -> Self {
        use ryo_execution_domain::domain::completion::types::CoverageState;
        match coverage {
            CoverageState::Complete => {}
            CoverageState::Partial { files, lacks } => {
                use ryo_execution_domain::domain::completion::service::{lacks_of, partial_of_lacks};
                let mut all = lacks_of(&files, &lacks);
                match &self.body.readiness {
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
                    Rc::make_mut(&mut self.body).readiness = Readiness::Partial { files, lacks };
                }
            }
            CoverageState::Inapplicable { files } => match &mut Rc::make_mut(&mut self.body).readiness {
                readiness @ Readiness::Complete => *readiness = Readiness::Inapplicable { files },
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

    pub(crate) fn validate_for(
        &self,
        owner: &Rc<RelationOwner>,
        generation: &Arc<ryo_execution_store::application::file_dag::relations::generation::service::GenerationLease>,
        required: &Partition,
        expected: ProducerRoot,
    ) -> Result<(), OperatingError> {
        if owner.matches_generation(generation) == ryo_execution_store::application::file_dag::relations::types::Sameness::Different
            || !Rc::ptr_eq(owner, &self.body.proof.owner)
            || ryo_execution_store::application::file_dag::relations::generation::service::GenerationLease::same(&self.body.proof.generation, generation) == ryo_execution_store::application::file_dag::relations::types::Sameness::Different
            || self.body.root != expected
            || self.body.partition.supplies(required) == PartitionSupply::Lacks
        {
            return Err(OperatingError::NotRead);
        }
        self.publication(owner)?;
        Ok(())
    }
    pub(crate) fn walk_source<'call>(
        &'call self,
        relation: ryo_relations::domain::relations::types::Relation,
        rows: &'call ryo_execution_domain::domain::table_model::types::Rows<'call>,
        generation: &Arc<ryo_execution_store::application::file_dag::relations::generation::service::GenerationLease>,
    ) -> Result<Option<AdmittedWalkSource<'call>>, OperatingError> {
        use ryo_execution_domain::domain::{
            table_model::{service::kinds_of, types::Rows},
        };
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
        if self.body.root != ProducerRoot::Input(InputSource::Relation(relation)) {
            return Err(OperatingError::WrongOperation);
        }
        if ryo_execution_store::application::file_dag::relations::generation::service::GenerationLease::same(&self.body.proof.generation, generation) == ryo_execution_store::application::file_dag::relations::types::Sameness::Different {
            return Err(OperatingError::NotRead);
        }
        let publication = self.publication(&self.body.proof.owner)?;
        let admitted = self
            .body
            .proof
            .owner
            .admit(publication.as_ref(), RepresentationInput::from(rows))?;
        let identity = admitted.identity(&self.body.proof.owner)?;
        let (population, actual_relation) = (identity.population(), identity.relation());
        // A project-stage input is the decoded frontier of the relation: computed rows carry no relation identity of their own, and
        // the root and the population proof already say which relation they are.
        if population != self.body.proof.population
            || actual_relation.is_some_and(|actual| actual != relation)
        {
            return Err(OperatingError::NotRead);
        }
        if kinds_of(rows) != ryo_relations::domain::relations::service::kinds(relation)
            || matches!(rows,Rows::Tree(tree) if ryo_execution_domain::domain::table_cells::service::native_form(relation) != Some(tree.form))
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
    pub(crate) fn from_publication(
        owner: Rc<RelationOwner>,
        publication: PublishedInput,
        admitted: &ryo_execution_store::application::file_dag::relations::admission::service::BorrowedInput<'_>,
        root: ProducerRoot,
        read: ReadProof,
    ) -> Result<Self, OperatingError> {
        admitted.require_publication(owner.as_ref(), publication.as_ref())?;
        let population = admitted.identity(&owner)?.population();
        let generation = admitted.generation();

        Ok(Self {
            body: Rc::new(AdmittedBody {
                proof: PopulationProof {
                    owner,
                    population,
                    generation,
                },
                root,
                partition: read.partition,
                readiness: read.readiness,
                publication,
            }),
        })
    }
    pub(crate) fn publication(
        &self,
        owner: &Rc<RelationOwner>,
    ) -> Result<&PublishedInput, OperatingError> {
        if !Rc::ptr_eq(owner, &self.body.proof.owner)
            || owner.matches_generation(&self.body.proof.generation) == ryo_execution_store::application::file_dag::relations::types::Sameness::Different
        {
            return Err(OperatingError::NotRead);
        }
        match &self.body.readiness {
            Readiness::Complete | Readiness::Partial { .. } | Readiness::Inapplicable { .. } => {
                Ok(&self.body.publication)
            }
        }
    }
}
