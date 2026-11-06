//! One public prepare/open/run protocol; ownership is never delegated to callers.
use super::types::*;
pub use crate::domain::readiness::types::ReadinessCounts;
pub use crate::domain::readiness::types::ReadinessEvents;
pub use crate::domain::shared_walk::types::WalkClassWork;
pub use crate::domain::shared_walk::types::WalkCounts;
pub use crate::domain::shared_walk::types::WalkPassCount;
pub use crate::domain::shared_walk::types::WalkPredicateRows;
pub use crate::domain::shared_walk::types::WalkSourceRows;
pub use crate::domain::shared_walk::types::{WalkDecodedRows, WalkPredicateTests};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::{
    files::types::CorpusInput, runs::types::Skipped, stored_facts::types::Bucket,
};
pub trait ExecutionUseCase {
    fn prepare(&self, request: PreparedRequest) -> Result<PreparedExecutionProgram, OperatingError>;
    fn prepare_compiled(
        &self,
        queries: CompiledQueries,
        rules: Vec<ryo_values::domain::rule_requests::types::RuleSpec>,
        digests: Vec<ryo_values::domain::stored_facts::types::Digest>,
        hosted: Vec<ryo_values::domain::hosted::types::HostRelation>,
        observations: ObservationIdentity,
    ) -> Result<PreparedExecutionProgram, OperatingError>;
    fn begin(&self, program: &PreparedExecutionProgram, corpus: &[CorpusInput]) -> RequestExecution;
}
pub trait ExecutionRequest {
    fn observations<'a>(
        &'a self,
        arenas: &'a mut Vec<ryo_values::domain::stored_facts::types::ArenaBytes>,
    ) -> Result<ObservationStage<'a>, OperatingError>;
    fn prepare_frontiers(
        &self,
        input: FrontierRequest<'_>,
        symbols: &mut Symbols,
    ) -> Result<PreparedFrontiers, PreparationError>;
    fn finish(self)
    where
        Self: Sized;
    fn open_file<'a, 'facts: 'a>(
        &'a self,
        input: FileInput<'a, 'facts>,
        symbols: &mut Symbols,
    ) -> Result<FileSession<'a, 'facts>, OperatingError>;
    /// One mandatory owner for all ordinary project rules in this request phase.
    fn project_phase(&self) -> Result<ProjectPhase<'_>, OperatingError>;
}
/// The intent supplied at file start is immutable throughout this session.
/// Applicability cannot be changed after another rule has started the shared walk.
/// ```compile_fail,E0599
/// use ryo_execution::inbound::use_cases::execution::{
///     interface::FileExecution,
///     types::{ApplicabilityReason, FileSession, RuleIndex},
/// };
/// fn late_skip(file: &mut FileSession<'_, '_>) {
///     file.skip_rule(RuleIndex(0), ApplicabilityReason::SyntaxRejected).unwrap();
/// }
/// ```
pub trait FileExecution {
    /// Actual bound-root readiness polling and dependency wakeups in this owner.
    fn readiness_counts(&self) -> ReadinessCounts;
    /// Actual shared native pass work already performed by this file owner.
    fn walk_counts(&self) -> WalkCounts;
    /// Deliver the already committed canonical publication to the callback.
    /// Sink refusal does not repeat admission or semantic evaluation.
    /// A different file-start delivery declaration is refused before source admission.
    fn emit_rows(
        &mut self,
        rule: RuleIndex,
        symbols: &mut Symbols,
        each: &mut dyn FnMut(
            &[ryo_relations::domain::relations::types::Value],
        ) -> Result<(), OperatingError>,
    ) -> Result<RowEmission, OperatingError>;
    fn finish(self) -> Result<FileFinished, OperatingError>
    where
        Self: Sized;
    /// Collect exactly the declared roots. Forwarded diagnostic roots cannot change mode.
    fn rule(
        &mut self,
        rule: RuleIndex,
        demand: RuleDemand,
        symbols: &mut Symbols,
    ) -> Result<RuleOutcome, OperatingError>;
    fn capture_frontiers(
        &mut self,
        rule: RuleIndex,
        first: Bucket,
        symbols: &mut Symbols,
        capture: CaptureContext<'_>,
    ) -> Result<Vec<FrontierCapture>, OperatingError>;
    fn commit_frontier(
        &mut self,
        rule: RuleIndex,
        input: ryo_values::domain::plan_parts::types::InputNumber,
    ) -> Result<(), OperatingError>;
    fn counts(&self, rule: RuleIndex) -> Result<LogicalCounts, OperatingError>;
}
pub use crate::domain::readiness::types::{InvocationStateCount, InvocationStateCounts};
/// Project rules are collected only; no row callback delivery exists on this route.
/// ```compile_fail,E0599
/// use ryo_execution::inbound::use_cases::execution::{
///     interface::{FileExecution, ProjectRootExecution, ProjectStage},
///     types::ProjectPhase,
/// };
/// fn deliver(project: &mut ProjectPhase<'_>) {
///     project.emit_rows();
/// }
/// ```
/// The authorized route, declared roots through the phase, compiles.
/// ```
/// use ryo_execution::inbound::use_cases::execution::{
///     interface::ProjectRootExecution,
///     types::{OperatingError, ProjectBatch, ProjectPhase, ProjectRuleOutcome, ProjectTask, RuleIndex},
/// };
/// use ryo_language::domain::symbols::types::Symbols;
/// fn evaluate(
///     phase: &ProjectPhase<'_>,
///     tasks: Vec<ProjectTask<'_>>,
///     symbols: &mut Symbols,
/// ) -> ProjectBatch<Result<ProjectRuleOutcome, OperatingError>, OperatingError> {
///     phase.evaluate(tasks, symbols, |_: RuleIndex| Ok(()))
/// }
/// ```
pub trait ProjectRootExecution {
    /// Bind all actual tasks, prepare compatible root cohorts, then deliver in declaration order.
    fn evaluate<E>(
        &self,
        tasks: Vec<ProjectTask<'_>>,
        symbols: &mut Symbols,
        guard: impl FnMut(RuleIndex) -> Result<(), E>,
    ) -> ProjectBatch<Result<ProjectRuleOutcome, OperatingError>, E>;
    fn readiness_counts(&self) -> ReadinessCounts;
}
pub trait ProjectStage: ProjectRootExecution {
    fn state_counts(&self) -> InvocationStateCounts;
    fn project_scope(
        &self,
        rule: RuleIndex,
        corpus: &[CorpusInput],
        covered: &[Covered],
        skipped: &[Skipped],
        inapplicable: &InapplicableFiles,
    ) -> Result<ProjectScope, OperatingError>;
    fn finish(self) -> Result<(), OperatingError>;
}

/// Read-only request metadata; no mutable program, permit or population escapes.
pub trait PreparedExecution {
    fn observation_batches(&self) -> Option<&[Vec<RuleIndex>]>;
    fn rule_count(&self) -> usize;
    fn rule_order(&self) -> impl Iterator<Item = RuleIndex> + '_;
    fn demanded_rules(&self, governs: impl Fn(usize) -> bool) -> Vec<bool>;
    fn emit(
        &self,
        rule: RuleIndex,
    ) -> Result<&ryo_values::domain::plans::types::Emit, OperatingError>;
    fn rule_digest(
        &self,
        rule: RuleIndex,
    ) -> Result<ryo_values::domain::stored_facts::types::Digest, OperatingError>;
    fn frontiers(
        &self,
        rule: RuleIndex,
    ) -> Result<&[ryo_values::domain::plan_parts::types::Input], OperatingError>;
    fn captures_observation(&self, rule: RuleIndex, input: usize) -> Result<bool, OperatingError>;
    fn semantic_needed(&self) -> bool;
    fn observation_layout(
        &self,
    ) -> Option<(
        ryo_values::domain::records::types::ObservationReadinessLayout,
        &[ObservedFrontierRoot],
    )>;
    fn observation_reads(&self, rule: RuleIndex) -> Result<Vec<RelationRead>, OperatingError>;
    fn has_observation_contract(&self, rule: RuleIndex) -> Result<bool, OperatingError>;
    fn frontier_dependencies(
        &self,
        rule: RuleIndex,
    ) -> Result<&[Vec<ryo_values::domain::plan_parts::types::ObservationBindingId>], OperatingError>;
}

pub trait ExecutionCompilation {
    fn compiling_into(self, cache: CompiledPlans) -> Self;
    fn compile(
        &self,
        rules: &[ryo_values::domain::rule_requests::types::RuleSpec],
        hosted: &[ryo_values::domain::hosted::types::HostRelation],
    ) -> Result<CompiledQueries, RuleCompilationError>;
    fn identify(
        &self,
        queries: &CompiledQueries,
        digest: &dyn digest::DigestPort,
        rules: Vec<ryo_values::domain::stores::types::StoredRule>,
    ) -> Result<PreparedRuleIdentities, RuleCompilationError>;
}
pub trait PreparedQueries {
    fn plans(&self) -> &[ryo_values::domain::plans::types::Plan];
    fn bindings(&self) -> &[Vec<ryo_values::domain::query_bindings::types::ParameterBinding>];
}

pub trait ObservationExecution {
    fn reserve(&mut self, rule: RuleIndex) -> Result<PublicationSlot, OperatingError>;
    fn bindings(&self, rule: RuleIndex) -> Result<Vec<BoundInput>, OperatingError>;
    fn binding_readiness(
        &self,
        rule: RuleIndex,
    ) -> Result<
        std::collections::BTreeMap<
            ryo_values::domain::plan_parts::types::ObservationBindingId,
            bool,
        >,
        OperatingError,
    >;
    fn arenas(&self) -> &[ryo_values::domain::stored_facts::types::ArenaBytes];
    fn project_scope(
        &self,
        rule: RuleIndex,
        corpus: &[CorpusInput],
        covered: &[Covered],
        skipped: &[Skipped],
        inapplicable: &InapplicableFiles,
    ) -> Result<ProjectScope, OperatingError>;
    fn finish(&mut self, rule: RuleIndex) -> Result<(), OperatingError>;
    fn finish_stage(self) -> Result<(), OperatingError>;
}

pub trait ObservationPublication {
    fn install(
        &mut self,
        rule: RuleIndex,
        slot: PublicationSlot,
        transfer: CrossPublicationTransfer,
        publications: Vec<(
            ryo_values::domain::records::types::ObservationIndex,
            ObservedPublication,
        )>,
    ) -> Result<(), OperatingError>;
    fn restore_frontiers(
        &mut self,
        facts: &dyn facts::FactsFinish,
        corpus: &[CorpusInput],
        covered: &[Covered],
        skipped: &[Skipped],
        inapplicable: &[InapplicableFiles],
        readiness: &[FileObservationReadiness],
        first_bucket: &[Bucket],
        selected: &[bool],
    ) -> Result<(), OperatingError>;
}

pub trait PreparedReporting {
    fn metadata(&self) -> &[ryo_values::domain::run_metadata::types::RuleRunMetadata];
    fn replay_closure(
        &self,
        again: ryo_values::domain::stores::types::Reuse,
        first: &[Bucket],
    ) -> Result<(ryo_values::domain::stores::types::Reuse, Vec<Bucket>), OperatingError>;
}
