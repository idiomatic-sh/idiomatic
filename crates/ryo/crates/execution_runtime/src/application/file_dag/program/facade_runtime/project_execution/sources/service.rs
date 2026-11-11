//! Deferred frontier sources of one project phase: their populations and invocation identity.
use crate::application::file_dag::frontiers::types::FrontierInput;
use crate::application::file_dag::program::admission::service::AvailableInput;
use crate::application::file_dag::program::admission::types::Partition;
use crate::application::file_dag::program::admission::types::ProducerRoot;
use crate::application::file_dag::program::facade_runtime::service::PublicationScope;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::project_population::types::BoundKey;
use crate::application::file_dag::program::project_population::types::ProjectFrontierPayload;
use crate::application::file_dag::program::project_population::types::ProjectFrontierPopulation;
use crate::application::file_dag::program::project_population::types::ProjectInvocationKey;
use crate::application::file_dag::program::project_population::types::ProjectRefusal;
use crate::application::file_dag::program::project_population::types::ProjectSourceMatch;
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::types::InputSource;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::relations::interface::RelationsPort as _;
use ryo_execution_store::application::file_dag::relations::service::RelationOwner;
use std::rc::Rc;
use std::sync::Arc;

impl ProjectFrontierPopulation<AvailableInput> {
    pub(crate) fn source(&self) -> &Result<InputSource, OperatingError> {
        &self.source
    }
    pub(crate) fn supply(
        &self,
        owner: &Rc<RelationOwner>,
        partition: &Partition,
    ) -> Result<AvailableInput, OperatingError> {
        self.admitted
            .get_or_init(|| {
                let source = self.source.clone()?;
                let publication = match &self.payload {
                    ProjectFrontierPayload::Flat(table) => {
                        owner.publish(ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody::Shared(Arc::clone(table)))
                    }
                    ProjectFrontierPayload::Compact(table) => {
                        owner.publish(ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody::Compact(Arc::clone(table)))
                    }
                }?;
                let unavailable = |_| Err(OperatingError::NotRead);
                let rows = owner.rows(&publication, &unavailable)?;
                AvailableInput::source(
                    Rc::clone(owner),
                    publication.clone(),
                    match source {
                        InputSource::Region(id) => ProducerRoot::Region(id),
                        _ => ProducerRoot::Input(source),
                    },
                    partition.clone(),
                    self.coverage.clone(),
                    rows,
                )
            })
            .clone()
    }
}

impl PublicationScope {
    pub(crate) fn sources(
        &self,
        owner: &Rc<RelationOwner>,
        sources: &[crate::application::file_dag::frontiers::types::FrontierSource],
        rows: &[FrontierInput],
    ) -> Result<Vec<Rc<ProjectFrontierPopulation<AvailableInput>>>, OperatingError> {
        rows.iter()
            .map(|row| {
                let (input, coverage, payload) = match row {
                    FrontierInput::Decoded { frontier, coverage } => (
                        frontier.input,
                        coverage.clone(),
                        ProjectFrontierPayload::Flat(Arc::clone(&frontier.table)),
                    ),
                    FrontierInput::Compact {
                        input,
                        table,
                        coverage,
                        ..
                    } => (
                        *input,
                        coverage.clone(),
                        ProjectFrontierPayload::Compact(Arc::clone(table)),
                    ),
                    // Admission precedes every source: a packed frontier here was never admitted.
                    FrontierInput::Packed(_) => return Err(OperatingError::NotRead),
                };
                let source = sources
                    .iter()
                    .find(|source| source.input == input)
                    .ok_or(OperatingError::NotRead)?
                    .root
                    .clone();
                let populations = &mut *self.populations.borrow_mut();
                populations.source(
                    |found| {
                        if found.source == source
                            && found.coverage == coverage
                            && found.payload == payload
                        {
                            ProjectSourceMatch::Equivalent
                        } else {
                            ProjectSourceMatch::Distinct
                        }
                    },
                    || {
                        Ok(ProjectFrontierPopulation {
                            id: owner.population()?,
                            source: source.clone(),
                            coverage: coverage.clone(),
                            payload: payload.clone(),
                            admitted: std::cell::OnceCell::new(),
                        })
                    },
                )
            })
            .collect()
    }
    pub(crate) fn invocation(
        &self,
        owner: &Rc<RelationOwner>,
        sources: &[Rc<ProjectFrontierPopulation<AvailableInput>>],
        refused: &[ryo_execution_domain::domain::frontiers::types::RefusedFrontier],
        frontier_sources: &[crate::application::file_dag::frontiers::types::FrontierSource],
        bound: &[Option<crate::application::file_dag::frontiers::types::BoundInput>],
        prepared: &PreparedProgram,
        rule: RuleIndex,
        policy: CompletionPolicy,
    ) -> Result<
        ryo_execution_store::application::file_dag::derived::population::service::PopulationId,
        OperatingError,
    > {
        let mut bindings = Vec::new();
        for binding in bound.iter().flatten() {
            bindings.push(BoundKey::Input(InputSource::Observation(ryo_execution_domain::domain::executable_program::types::ObservationPlace(
                crate::application::file_dag::program::facade_runtime::source_admission::service::observation_slot(prepared, rule, binding.input)?.0,
            ))));
            match &binding.members {
                Err(error) => bindings.push(BoundKey::Refused(error.clone())),
                Ok(members) => {
                    for member in members {
                        bindings.push(match member {
                        crate::application::file_dag::frontiers::types::BoundMember::Current(publication) => BoundKey::Current(publication.population(), publication.coverage().clone()),
                        crate::application::file_dag::frontiers::types::BoundMember::Restored(token) => BoundKey::Restored(token.clone()),
                    });
                    }
                }
            }
        }
        let key = ProjectInvocationKey {
            sources: sources.iter().map(|source| source.id).collect(),
            refused: refused
                .iter()
                .map(|entry| ProjectRefusal {
                    source: frontier_sources
                        .iter()
                        .find(|source| source.input == entry.input)
                        .map(|source| source.root.clone()),
                    input: entry.input,
                    error: entry.error.clone(),
                })
                .collect(),
            policy,
            bound: bindings,
        };
        self.populations
            .borrow_mut()
            .invocation(key, || owner.population())
    }
}
