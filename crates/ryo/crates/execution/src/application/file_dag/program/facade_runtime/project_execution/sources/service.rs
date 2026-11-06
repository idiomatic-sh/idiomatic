//! Deferred frontier sources of one project phase: their populations and invocation identity.
use crate::application::file_dag::frontiers::types::FrontierInput;
use crate::application::file_dag::program::admission::AvailableInput;
use crate::application::file_dag::program::admission::Partition;
use crate::application::file_dag::program::admission::ProducerRoot;
use crate::application::file_dag::program::facade_runtime::types::PublicationScope;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::project_population::types::BoundKey;
use crate::application::file_dag::program::project_population::types::ProjectFrontierPayload;
use crate::application::file_dag::program::project_population::types::ProjectFrontierPopulation;
use crate::application::file_dag::program::project_population::types::ProjectInvocationKey;
use crate::application::file_dag::program::project_population::types::ProjectRefusal;
use crate::application::file_dag::program::project_population::types::ProjectSourceMatch;
use crate::application::file_dag::relations::RelationOwner;
use crate::domain::completion::types::completion_policy::value::CompletionPolicy;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::executable_program::types::InputSource;
use crate::domain::operating::errors::OperatingError;
use std::rc::Rc;
use std::sync::Arc;

impl ProjectFrontierPopulation<AvailableInput> {
    pub(in crate::application::file_dag::program::facade_runtime) fn source(
        &self,
    ) -> &Result<InputSource, OperatingError> {
        &self.source
    }
    pub(in crate::application::file_dag::program::facade_runtime) fn supply(
        &self,
        owner: &Rc<RelationOwner>,
        partition: &Partition,
    ) -> Result<AvailableInput, OperatingError> {
        self.admitted
            .get_or_init(|| {
                let source = self.source.clone()?;
                let publication = match &self.payload {
                    ProjectFrontierPayload::Flat(table) => {
                        owner.source_shared_computed(Arc::clone(table))
                    }
                    ProjectFrontierPayload::Compact(table) => {
                        owner.source_compact(Arc::clone(table))
                    }
                }?;
                let unavailable = |_| Err(OperatingError::NotRead);
                let rows = owner.resolve(publication.as_ref(), &unavailable)?;
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
    pub(in crate::application::file_dag::program::facade_runtime::project_execution) fn sources(
        &self,
        owner: &Rc<RelationOwner>,
        sources: &[crate::application::file_dag::frontiers::types::FrontierSource],
        rows: &[crate::application::file_dag::frontiers::types::FrontierInput],
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
                        input,
                        coverage,
                        ProjectFrontierPayload::Compact(Arc::new(table)),
                    ),
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
                            id: owner.project_identity()?,
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
    pub(in crate::application::file_dag::program::facade_runtime::project_execution) fn invocation(
        &self,
        owner: &Rc<RelationOwner>,
        sources: &[Rc<ProjectFrontierPopulation<AvailableInput>>],
        refused: &[crate::domain::crossing::types::RefusedFrontier],
        frontier_sources: &[crate::application::file_dag::frontiers::types::FrontierSource],
        bound: &[Option<crate::application::file_dag::frontiers::types::BoundInput>],
        prepared: &PreparedProgram,
        rule: RuleIndex,
        policy: CompletionPolicy,
    ) -> Result<
        crate::application::file_dag::derived::population::types::PopulationId,
        OperatingError,
    > {
        let mut bindings = Vec::new();
        for binding in bound.iter().flatten() {
            bindings.push(BoundKey::Input(InputSource::Observation(
                crate::application::file_dag::program::facade_runtime::source_admission::observation_slot(prepared, rule, binding.input)?.0,
            )));
            match &binding.members {
                Err(error) => bindings.push(BoundKey::Refused(error.clone())),
                Ok(members) => {
                    for member in members {
                        bindings.push(match member {
                        crate::application::file_dag::frontiers::types::BoundMember::Current(publication) => BoundKey::Current(publication.admitted().population(owner)?, publication.admitted().coverage()?),
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
            .invocation(key, || owner.project_identity())
    }
}
