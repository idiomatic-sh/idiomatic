//! The admitted input: identity, rows, coverage and generation validation.
use super::super::{
    service::AdmittedInput, service::AdmittedWalkSource, service::Partition,
    service::PopulationProof, service::ReadProof, types::Agreement, types::ObservationReadiness,
    types::PartitionSupply, types::ProducerRoot,
};
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::{
    executable_program::types::InputSource, operating::errors::OperatingError,
};
use ryo_execution_store::application::file_dag::relations::interface::RelationsPort as _;
use ryo_execution_store::application::file_dag::relations::{
    handle::service::Publication, service::RelationOwner,
};
use std::rc::Rc;

impl ReadProof {
    /// A whole physical read of `partition`.
    pub(crate) fn complete(partition: Partition) -> Self {
        Self {
            partition,
            readiness: CoverageState::Complete,
        }
    }
    /// A read of `partition` whose coverage the completion domain computed.
    pub(crate) fn covered(partition: Partition, coverage: CoverageState) -> Self {
        Self {
            partition,
            readiness: coverage,
        }
    }
    /// How completely the read covered its partition.
    pub(crate) fn coverage(&self) -> &CoverageState {
        &self.readiness
    }
    pub(crate) fn observation(&self) -> ObservationReadiness {
        match self.readiness {
            CoverageState::Complete => ObservationReadiness::Ready,
            CoverageState::Partial { .. } | CoverageState::Inapplicable { .. } => {
                ObservationReadiness::Waiting
            }
        }
    }
}

impl AdmittedInput {
    /// What the input's read proved: its partition and how completely it was read.
    pub(crate) fn read_proof(&self) -> ReadProof {
        ReadProof {
            partition: self.partition.clone(),
            readiness: self.readiness.clone(),
        }
    }
    /// The producer root the input was admitted for.
    pub(crate) fn producer_root(&self) -> ProducerRoot {
        self.root
    }
    /// The publication the input holds, whatever the owner.
    pub(crate) fn held_publication(&self) -> &Publication {
        &self.publication
    }
    /// The population the input was admitted under.
    pub(crate) fn admitted_population(
        &self,
    ) -> ryo_execution_store::application::file_dag::derived::population::service::PopulationId
    {
        self.proof.population
    }
    /// The generation the input was admitted under.
    pub(crate) fn admitted_generation(
        &self,
    ) -> &ryo_execution_store::application::file_dag::relations::generation::service::ResidentGeneration{
        &self.proof.generation
    }
    pub(crate) fn population(
        &self,
        owner: &Rc<RelationOwner>,
    ) -> Result<
        ryo_execution_store::application::file_dag::derived::population::service::PopulationId,
        OperatingError,
    > {
        self.publication(owner)?;
        Ok(self.proof.population)
    }
    pub(crate) fn has_root(&self, root: ProducerRoot) -> Agreement {
        if self.root == root {
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
        ) -> Result<
            ryo_execution_domain::domain::table_model::types::Rows<'facts>,
            OperatingError,
        >,
    ) -> Result<ryo_execution_domain::domain::table_model::types::Rows<'a>, OperatingError>
    where
        'facts: 'a,
    {
        let publication = self.publication(owner)?;
        let rows = owner.rows(publication, read)?;
        owner.normalize(publication, rows)
    }
    pub(crate) fn observation_ready(&self) -> Result<ObservationReadiness, OperatingError> {
        match &self.readiness {
            CoverageState::Complete => Ok(ObservationReadiness::Ready),
            CoverageState::Partial { .. } | CoverageState::Inapplicable { .. } => {
                Ok(ObservationReadiness::Waiting)
            }
        }
    }

    /// What the input's read covered.
    pub(crate) fn coverage(&self) -> &CoverageState {
        &self.readiness
    }
    pub(crate) fn with_coverage(mut self, coverage: CoverageState) -> Self {
        match coverage {
            CoverageState::Complete => {}
            CoverageState::Partial { files, lacks } => {
                use ryo_execution_domain::domain::completion::service::{
                    lacks_of, partial_of_lacks,
                };
                let mut all = lacks_of(&files, &lacks);
                match &self.readiness {
                    CoverageState::Complete => {}
                    CoverageState::Partial {
                        files: existing,
                        lacks: existing_lacks,
                    } => all.extend(lacks_of(existing, existing_lacks)),
                    CoverageState::Inapplicable { files: existing } => {
                        all.extend(lacks_of(
                            &existing.iter().copied().map(Some).collect::<Vec<_>>(),
                            &[],
                        ));
                    }
                }
                if let CoverageState::Partial { files, lacks } = partial_of_lacks(all) {
                    self.readiness = CoverageState::Partial { files, lacks };
                }
            }
            CoverageState::Inapplicable { files } => match &mut self.readiness {
                CoverageState::Complete => self.readiness = CoverageState::Inapplicable { files },
                CoverageState::Partial {
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
                CoverageState::Inapplicable { files: existing } => {
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
        generation: &ryo_execution_store::application::file_dag::relations::generation::service::ResidentGeneration,
        required: &Partition,
        expected: ProducerRoot,
    ) -> Result<(), OperatingError> {
        if owner.generation().same(generation).is_err()
            || !Rc::ptr_eq(owner, &self.proof.owner)
            || self.proof.generation.same(generation).is_err()
            || self.root != expected
            || self.partition.supplies(required) == PartitionSupply::Lacks
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
        generation: &ryo_execution_store::application::file_dag::relations::generation::service::ResidentGeneration,
    ) -> Result<Option<AdmittedWalkSource<'call>>, OperatingError> {
        use ryo_execution_domain::domain::table_model::types::RepresentationInput;
        use ryo_execution_domain::domain::table_model::{service::kinds_of, types::Rows};
        if self.root != ProducerRoot::Input(InputSource::Relation(relation)) {
            return Err(OperatingError::WrongOperation);
        }
        if self.proof.generation.same(generation).is_err() {
            return Err(OperatingError::NotRead);
        }
        let publication = self.publication(&self.proof.owner)?;
        let admitted = self
            .proof
            .owner
            .admit(publication, RepresentationInput::from(rows))??;
        let identity = admitted.identity(&self.proof.owner)?;
        let (population, actual_relation) = (identity.population(), identity.relation());
        // A project-stage input is the decoded frontier of the relation: computed rows carry no relation identity of their own, and
        // the root and the population proof already say which relation they are.
        if population != self.proof.population
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
        publication: Publication,
        admitted: &ryo_execution_store::application::file_dag::relations::admission::service::BorrowedInput<'_>,
        root: ProducerRoot,
        read: ReadProof,
    ) -> Result<Self, OperatingError> {
        admitted.require_publication(owner.as_ref(), &publication)?;
        let population = admitted.identity(&owner)?.population();
        let generation = admitted.generation();

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
    pub(crate) fn publication(
        &self,
        owner: &Rc<RelationOwner>,
    ) -> Result<&Publication, OperatingError> {
        if !Rc::ptr_eq(owner, &self.proof.owner)
            || owner.generation().same(&self.proof.generation).is_err()
        {
            return Err(OperatingError::NotRead);
        }
        match &self.readiness {
            CoverageState::Complete
            | CoverageState::Partial { .. }
            | CoverageState::Inapplicable { .. } => Ok(&self.publication),
        }
    }
}
