//! File execution and named observation lifecycle.
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::admission::FileGeneration;
use crate::application::file_dag::program::admission::Partition;
use crate::application::file_dag::derived::constants::TARGET_RESOURCE_ROUTES;
use crate::application::file_dag::program::constants::{
    FILE_OWNER_FINISH, PRODUCER_COVERAGE_INCOMPLETE,
};
use crate::application::file_dag::program::facade_runtime::source_admission::table_of;
use crate::application::file_dag::program::facade_runtime::types::ProgramServices;
use crate::application::file_dag::program::facade_runtime::types::{
    FileRoot, FrontierCommit, ObservationKey, PreparedNamed, WalkPreparation,
};
use crate::application::file_dag::relations::types::ReuseEligibility;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::domain::rule_intent::types::FileRuleIntent;
use crate::domain::operator::types::LogicalCounts;
#[cfg(test)]
use crate::application::file_dag::program::rule_outcome::types::ProjectObservation;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::application::file_dag::program::runtime::service::InvocationOutcome;
use crate::application::file_dag::program::runtime::service::ProgramExecution;
use crate::application::file_dag::relations::RelationOwner;
use crate::application::file_dag::scope::ScopePartials;
use crate::application::file_dag::scope::open_program_owner;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::operating::errors::OperatingError;
#[cfg(test)]
use crate::domain::table_model::service::materialize;
use crate::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::files::types::SourceView;
#[cfg(test)]
use ryo_values::domain::rows::types::Table;
use rustc_hash::FxHashMap;
use std::rc::Rc;
use std::sync::Arc;

#[cfg(test)]
use crate::application::file_dag::program::admission::Inputs;
#[cfg(test)]
use crate::application::file_dag::program::facade_runtime::source_admission::SourceInputs;
#[cfg(test)]
use crate::domain::rule_intent::types::CaptureIntent;
#[cfg(test)]
use crate::domain::rule_intent::types::DiagnosticDelivery;
#[cfg(test)]
use crate::domain::rule_intent::types::EmitDemand;
#[cfg(test)]
use crate::domain::rule_intent::types::ObservationDemand;
#[cfg(test)]
use crate::domain::rule_intent::types::RuleDemand;
#[cfg(test)]
use crate::domain::completion::types::completion_policy::value::CompletionPolicy;
#[cfg(test)]
use crate::domain::operating::types::OperatorContext;

mod emission;
mod frontier;
mod rule;
mod walk;

pub(crate) struct FileRun<'a> {
    program: &'a Arc<PreparedProgram>,
    services: Arc<ProgramServices>,
    execution: ProgramExecution,
    owner: Rc<RelationOwner>,
    file: FileId,
    partials: ScopePartials<'a>,
    roots: FxHashMap<RuleIndex, Vec<FileRoot>>,
    prepared_roots: Vec<crate::domain::executable_program::types::RegionId>,
    prepared_named: Vec<PreparedNamed>,
    /// Immutable intent and identity share the same private file owner lifetime.
    intents: Box<[FileRuleIntent]>,
    generation: FileGeneration,
    observed: FxHashMap<ObservationKey, AdmittedInput>,
    context: super::super::context::RunContext<'a>,
    walk: super::super::native_walk::FileWalk,
    walk_prepared: WalkPreparation,
    routes: crate::application::file_dag::route_counts::types::FileScope,
}
pub(crate) fn open_file<'a>(
    program: &'a Arc<PreparedProgram>,
    file: FileId,
    partials: ScopePartials<'a>,
    intents: &[FileRuleIntent],
    services: Arc<ProgramServices>,
    source: SourceView<'a>,
    reuse_eligible: ReuseEligibility,
    symbols: &mut ryo_language::domain::symbols::types::Symbols,
    semantic: Option<&'a dyn crate::domain::operating::types::SemanticLookup>,
    kinds: Option<Arc<crate::domain::kind_sets::types::vocabulary::value::PreparedVocabulary>>,
) -> Result<FileRun<'a>, OperatingError> {
    if intents.len() != program.rule_count().0 {
        return Err(OperatingError::ShapeMismatch);
    }
    if services.request.contains(file)
        == crate::application::file_dag::program::request_identity::CorpusMembership::Unlisted
    {
        return Err(OperatingError::NotRead);
    }
    if !matches!(partials, ScopePartials::File { .. }) {
        return Err(OperatingError::WrongOperation);
    }
    let routes = crate::application::file_dag::route_counts::types::FileScope::begin(file.0 as usize);
    #[cfg(feature = "diagnostic-row-progress")]
    if let Some(path) = services.request.path(file) {
        crate::application::file_dag::route_counts::progress::service::file_name(
            routes.profile(),
            path,
        );
    }
    let owner = open_program_owner(
        program.program(),
        services.derived().clone(),
        reuse_eligible,
        routes.profile().clone(),
    );
    let ScopePartials::File { publication, .. } = partials else {
        return Err(OperatingError::WrongOperation);
    };
    let generation = FileGeneration::from_owner(file, &owner);
    let execution = ProgramExecution::open(
        Arc::clone(program.program()),
        Partition::File(generation.clone()),
        Rc::clone(&owner),
    );
    let mut run = FileRun {
        routes,
        program,
        services: Arc::clone(&services),
        execution,
        owner,
        file,
        partials,
        roots: FxHashMap::default(),
        prepared_roots: Vec::new(),
        prepared_named: Vec::new(),
        intents: intents.into(),
        generation: generation.clone(),
        observed: FxHashMap::default(),
        walk: super::super::native_walk::FileWalk::new(generation),
        walk_prepared: WalkPreparation::Pending,
        context: super::super::context::RunContext::file(
            &services, source, symbols, semantic, kinds,
        ),
    };
    let demanded = run.bind_intents()?;
    run.owner
        .derived()
        .prepare_program_file(program.derived(), &demanded, publication)?;
    Ok(run)
}
impl FileRun<'_> {
    pub(crate) fn finish(
        mut self,
    ) -> Result<Option<Arc<ryo_values::domain::records::types::FrontierReadiness>>, OperatingError>
    {
        let readiness = self.observation_snapshot();
        self.roots.clear();
        self.observed.clear();
        self.walk.retire_all();
        self.owner.derived().report(self.file);
        let crate::application::file_dag::derived::owner::types::BuildCounts(
            builds,
            hits,
            refusals,
        ) = self.owner.derived().counts();
        tracing::trace!(target: TARGET_RESOURCE_ROUTES, file = self.file.0, builds = builds.0, hits = hits.0, refusals = refusals.0, event = FILE_OWNER_FINISH);
        self.execution.finish()?;
        drop(self.routes);
        Ok(readiness)
    }
    /// Weak lifetime observation only; the test never retains a workspace or owner.
    #[cfg(test)]
    pub(crate) fn owner_lifetime(&self) -> std::rc::Weak<RelationOwner> {
        Rc::downgrade(&self.owner)
    }
    /// Read-only test telemetry from actual common-program dispatch and retirement.
    #[cfg(test)]
    pub(crate) fn route_observation(
        &self,
        rule: RuleIndex,
        step: ryo_values::domain::plan_parts::types::StepId,
    ) -> Result<crate::domain::operating::fixtures::ExecutionObservation, OperatingError> {
        self.execution
            .route_observation(self.execution.file_state(), rule, step)
    }
    #[cfg(test)]
    pub(crate) fn resource_fixture_counts(
        &self,
    ) -> crate::application::file_dag::derived::owner::fixtures::resources::Snapshot {
        self.owner.derived().resource_fixture_counts()
    }
    #[cfg(test)]
    pub(crate) fn observation_routes(
        &self,
        slot: ryo_values::domain::records::types::ObservationIndex,
    ) -> Result<crate::domain::operating::fixtures::ExecutionObservation, OperatingError> {
        let region = self
            .program
            .program()
            .observation_regions()
            .get(slot.0 as usize)
            .ok_or(OperatingError::NotRun)?
            .as_ref()
            .map_err(Clone::clone)?;
        self.execution
            .region_observation(self.execution.file_state(), *region)
    }
    pub(crate) fn readiness_counts(&self) -> crate::domain::readiness::types::ReadinessCounts {
        self.execution.readiness_counts()
    }
    pub(crate) fn walk_counts(&self) -> crate::domain::shared_walk::types::WalkCounts {
        self.walk.counts()
    }
    pub(crate) fn counts(&self, rule: RuleIndex) -> Result<LogicalCounts, OperatingError> {
        self.execution
            .logical_counts(self.execution.file_state(), rule)
    }
    pub(crate) fn observation_snapshot(
        &self,
    ) -> Option<Arc<ryo_values::domain::records::types::FrontierReadiness>> {
        use ryo_values::domain::observations::service::{new_frontiers, set_frontier};
        use ryo_values::domain::records::types::ObservationReadinessCount;
        use ryo_values::domain::records::types::ObservationReadinessSlot;
        use ryo_values::domain::records::types::ProducerReadiness;
        let metadata = self.program.observation_metadata()?;
        if metadata.frontiers.is_empty() {
            return None;
        }
        let Ok(count) = u32::try_from(metadata.frontiers.len()) else {
            return None;
        };
        let mut map = new_frontiers(metadata.layout, ObservationReadinessCount(count));
        for (slot, root) in metadata.frontiers.iter().enumerate() {
            let Some(state) = self
                .roots
                .get(&root.producer)
                .and_then(|outputs| outputs.get(root.output.0 as usize))
            else {
                continue;
            };
            let readiness = match &state.readiness {
                Some(ProducerReadiness::Complete) if state.commit == FrontierCommit::Committed => {
                    ProducerReadiness::Complete
                }
                Some(ProducerReadiness::Complete) => continue,
                Some(other) => other.clone(),
                None => continue,
            };
            let Ok(position) = u32::try_from(slot) else {
                return None;
            };
            if set_frontier(&mut map, ObservationReadinessSlot(position), readiness).is_err() {
                return None;
            }
        }
        Some(Arc::new(map))
    }
}

fn readiness_of(
    outcome: &RootOutcome,
) -> Option<ryo_values::domain::records::types::ProducerReadiness> {
    use ryo_values::domain::records::types::ObservationReason;
    use ryo_values::domain::records::types::ProducerReadiness;
    Some(match outcome {
        RootOutcome::Complete(_) => ProducerReadiness::Complete,
        RootOutcome::Partial { .. } => {
            ProducerReadiness::Unknown(ObservationReason(PRODUCER_COVERAGE_INCOMPLETE.into()))
        }
        RootOutcome::Skipped(_) => ProducerReadiness::Skipped,
        RootOutcome::Unknown(error) | RootOutcome::Failed(error) => {
            ProducerReadiness::Unknown(ObservationReason(error.to_string().into()))
        }
        RootOutcome::NoQuery => ProducerReadiness::NoQuery,
        RootOutcome::Deferred { .. } => return None,
    })
}

pub(super) fn classify(error: OperatingError) -> RootOutcome {
    match error {
        OperatingError::ObservationIncomplete { .. } | OperatingError::SemanticIncomplete { .. } => {
            RootOutcome::Unknown(error)
        }
        other => RootOutcome::Failed(other),
    }
}
pub(super) fn outcome_of<'facts>(
    result: Result<InvocationOutcome, OperatingError>,
    owner: &Rc<RelationOwner>,
    read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
) -> RootOutcome {
    match result {
        Ok(InvocationOutcome::Published(input)) => match table_of(owner, &input, read) {
            Ok(table) => match input.coverage() {
                Ok(crate::domain::completion::types::CoverageState::Complete) => {
                    RootOutcome::Complete(table)
                }
                Ok(
                    coverage @ (crate::domain::completion::types::CoverageState::Partial { .. }
                    | crate::domain::completion::types::CoverageState::Inapplicable {
                        ..
                    }),
                ) => RootOutcome::Partial {
                    table,
                    affected: crate::domain::frontier_coverage::service::affected_files(&coverage)
                        .unwrap_or_default(),
                },
                Err(error) => classify(error),
            },
            Err(error) => classify(error),
        },
        Ok(InvocationOutcome::Ready | InvocationOutcome::Suspended(_)) => {
            RootOutcome::Failed(OperatingError::NotRun)
        }
        Err(error) => classify(error),
    }
}

#[cfg(test)]
mod distinct_integration;

#[cfg(test)]
impl FileRun<'_> {
    pub(crate) fn source_reuse_eligible(&self) -> ReuseEligibility {
        self.owner.stable()
    }
}
