//! The `file_execution` application operation.

use crate::application::file_dag::derived::service::prepare_file;
use crate::application::file_dag::program::admission::service::AdmittedInput;
use crate::application::file_dag::program::admission::service::FileGeneration;
#[cfg(test)]
use crate::application::file_dag::program::admission::service::Inputs;
use crate::application::file_dag::program::admission::service::Partition;
use crate::application::file_dag::program::constants::{
    FILE_OWNER_FINISH, PRODUCER_COVERAGE_INCOMPLETE,
};
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
#[cfg(test)]
use crate::application::file_dag::program::facade_runtime::source_admission::service::SourceInputs;
use crate::application::file_dag::program::facade_runtime::source_admission::service::table_of;
use crate::application::file_dag::program::facade_runtime::types::{
    FileRoot, FrontierCommit, ObservationKey, PreparedNamed, WalkPreparation,
};
use crate::application::file_dag::program::preparation::types::PreparedProgram;
#[cfg(test)]
use crate::application::file_dag::program::rule_outcome::types::ProjectObservation;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::application::file_dag::program::runtime::service::InvocationOutcome;
use crate::application::file_dag::program::runtime::service::ProgramExecution;
use rustc_hash::FxHashMap;
#[cfg(test)]
use ryo_execution_domain::domain::completion::types::completion_policy::value::CompletionPolicy;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::types::LogicalCounts;
#[cfg(test)]
use ryo_execution_domain::domain::operator::types::OperatorContext;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_store::application::file_dag::derived::constants::TARGET_RESOURCE_ROUTES;
use ryo_execution_store::application::file_dag::relations::service::RelationOwner;
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_execution_store::application::file_dag::service::open_program_owner;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::files::types::SourceView;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct FileRun<'a> {
    pub(super) program: &'a Arc<PreparedProgram>,
    pub(super) services: Arc<ProgramServices>,
    pub(super) execution: ProgramExecution,
    pub(super) owner: Rc<RelationOwner>,
    pub(super) file: FileId,
    pub(super) partials: ScopePartials<'a>,
    pub(super) roots: FxHashMap<RuleIndex, Vec<FileRoot>>,
    pub(super) prepared_roots:
        Vec<ryo_execution_domain::domain::executable_program::types::RegionId>,
    pub(super) prepared_named: Vec<PreparedNamed>,
    /// Immutable intent and identity share the same private file owner lifetime.
    pub(super) intents: Box<[FileRuleIntent]>,
    pub(super) generation: FileGeneration,
    pub(super) observed: FxHashMap<ObservationKey, AdmittedInput>,
    pub(super) context: super::super::super::context::service::RunContext<'a>,
    pub(super) walk: super::super::super::native_walk::service::FileWalk,
    pub(super) walk_prepared: WalkPreparation,
    pub(super) routes:
        ryo_execution_store::application::file_dag::route_counts::profile::service::FileScope,
    pub(super) presence:
        crate::application::file_dag::program::facade_runtime::service::FilePresence,
    pub(super) source: SourceView<'a>,
    /// The file's selected roots, bound before any region is touched: presence reads the file's rows, so it runs when the
    /// first entry point admits a source, never when the file opens.
    pub(super) activation: Option<super::super::types::PendingActivation>,
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
    semantic: Option<&'a dyn ryo_execution_domain::domain::operating::types::SemanticLookup>,
    kinds: Option<
        Arc<ryo_execution_domain::domain::kind_sets::types::vocabulary::value::PreparedVocabulary>,
    >,
) -> Result<FileRun<'a>, OperatingError> {
    if intents.len() != program.rule_count().0 {
        return Err(OperatingError::ShapeMismatch);
    }
    if services.request.contains(file)
        == crate::application::file_dag::program::request_identity::types::CorpusMembership::Unlisted
    {
        return Err(OperatingError::NotRead);
    }
    if !matches!(partials, ScopePartials::File { .. }) {
        return Err(OperatingError::WrongOperation);
    }
    let routes = ryo_execution_store::application::file_dag::route_counts::profile::service::FileScope::begin(
        file.0 as usize,
        services.derived().routes(),
    );
    #[cfg(feature = "diagnostic-row-progress")]
    if let Some(path) = services.request.path(file) {
        routes.profile().file_name(path);
    }
    let owner = open_program_owner(
        ryo_execution_store::application::file_dag::relations::types::ProducerCapacity(
            program.program().regions().len(),
        ),
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
        walk: super::super::super::native_walk::service::FileWalk::new(generation),
        walk_prepared: WalkPreparation::Pending,
        context: super::super::super::context::service::RunContext::file(
            &services, source, symbols, semantic, kinds,
        ),
        presence: crate::application::file_dag::program::facade_runtime::service::FilePresence {
            gate: super::super::types::PresenceGate::On,
            tests: Vec::new(),
            formulas: Vec::new(),
            read: Vec::new(),
            cells: Vec::new(),
            open: Vec::new(),
            memo: Vec::new(),
            scratch: Default::default(),
        },
        source,
        activation: None,
    };
    let demanded = run.bind_intents()?;
    prepare_file(&run.owner, program.derived(), &demanded, publication)?;
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
        let ryo_execution_store::application::file_dag::derived::owner::types::BuildCounts(
            builds,
            hits,
            refusals,
        ) = self.owner.derived().counts();
        tracing::trace!(target: TARGET_RESOURCE_ROUTES, file = self.file.0, builds = builds.0, hits = hits.0, refusals = refusals.0, event = FILE_OWNER_FINISH);
        self.execution.finish()?;
        drop(self.routes);
        Ok(readiness)
    }
    /// Whether the file's cleanup has recorded a refusal.
    #[cfg(test)]
    pub(crate) fn cleanup_status(&self) -> Result<(), OperatingError> {
        self.execution.cleanup_status()
    }
    /// The live state the file's execution holds.
    #[cfg(test)]
    pub(crate) fn state_counts(
        &self,
    ) -> ryo_execution_domain::domain::readiness::types::InvocationStateCounts {
        self.execution.state_counts()
    }
    /// Weak lifetime observation only; the test never retains a workspace or owner.
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) fn owner_lifetime(&self) -> std::rc::Weak<RelationOwner> {
        Rc::downgrade(&self.owner)
    }
    /// Read-only test telemetry from actual common-program dispatch and retirement.
    #[cfg(test)]
    pub(crate) fn route_observation(
        &self,
        rule: RuleIndex,
        step: ryo_values::domain::plan_parts::types::StepId,
    ) -> Result<
        ryo_execution_domain::domain::operating::fixtures::ExecutionObservation,
        OperatingError,
    > {
        self.execution
            .route_observation(self.execution.file_state(), rule, step)
    }
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) fn resource_fixture_counts(
        &self,
    ) -> ryo_execution_store::application::file_dag::derived::owner::fixtures::resources::Snapshot
    {
        self.owner.derived().resource_fixture_counts()
    }
    #[cfg(test)]
    pub(crate) fn observation_routes(
        &self,
        slot: ryo_values::domain::records::types::ObservationIndex,
    ) -> Result<
        ryo_execution_domain::domain::operating::fixtures::ExecutionObservation,
        OperatingError,
    > {
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
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) fn readiness_counts(
        &self,
    ) -> ryo_execution_domain::domain::readiness::types::ReadinessCounts {
        self.execution.readiness_counts()
    }
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) fn walk_counts(
        &self,
    ) -> ryo_execution_domain::domain::shared_walk::types::WalkCounts {
        self.walk.counts()
    }
    /// Reports the rows each step of `rule` was fed to the request's observer, after the checks every count read makes;
    /// without an observer only the checks are made.
    pub(crate) fn observe(&self, rule: RuleIndex) -> Result<(), OperatingError> {
        use crate::application::file_dag::program::runtime::types::FedRead;
        let observer = self.services.observer();
        let counts = self.counts(
            rule,
            if observer.is_some() {
                FedRead::Collect
            } else {
                FedRead::CheckOnly
            },
        )?;
        if let Some(observer) = observer {
            observer.fed(rule, counts)?;
        }
        Ok(())
    }
    /// The rows each step of `rule` was fed, for the interpreter's own proofs.
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) fn collected_counts(
        &self,
        rule: RuleIndex,
    ) -> Result<LogicalCounts, OperatingError> {
        self.counts(
            rule,
            crate::application::file_dag::program::runtime::types::FedRead::Collect,
        )
    }
    pub(crate) fn counts(
        &self,
        rule: RuleIndex,
        read: crate::application::file_dag::program::runtime::types::FedRead,
    ) -> Result<LogicalCounts, OperatingError> {
        self.execution
            .logical_counts(self.execution.file_state(), rule, read)
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

pub(super) fn readiness_of(
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

pub(crate) fn classify(error: OperatingError) -> RootOutcome {
    match error {
        OperatingError::ObservationIncomplete { .. }
        | OperatingError::SemanticIncomplete { .. } => RootOutcome::Unknown(error),
        other => RootOutcome::Failed(other),
    }
}

pub(crate) fn outcome_of<'facts>(
    result: Result<InvocationOutcome, OperatingError>,
    owner: &Rc<RelationOwner>,
    read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
) -> RootOutcome {
    match result {
        Ok(InvocationOutcome::Published(input)) => match table_of(owner, &input, read) {
            Ok(table) => match input.coverage() {
                ryo_execution_domain::domain::completion::types::CoverageState::Complete => {
                    RootOutcome::Complete(table)
                }
                coverage @ (ryo_execution_domain::domain::completion::types::CoverageState::Partial { .. }
                | ryo_execution_domain::domain::completion::types::CoverageState::Inapplicable {
                    ..
                }) => RootOutcome::Partial {
                    table,
                    affected: ryo_execution_domain::domain::frontier_coverage::service::affected_files(coverage)
                        .unwrap_or_default(),
                },
            },
            Err(error) => classify(error),
        },
        Ok(InvocationOutcome::Ready | InvocationOutcome::Suspended(_)) => {
            RootOutcome::Failed(OperatingError::NotRun)
        }
        Err(error) => classify(error),
    }
}

#[cfg(any(test, feature = "fixtures"))]
impl FileRun<'_> {
    pub(crate) fn source_reuse_eligible(&self) -> ReuseEligibility {
        self.owner.stable()
    }
}
