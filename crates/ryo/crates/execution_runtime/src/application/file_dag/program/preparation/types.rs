//! Immutable prepared request metadata; no runtime or lifecycle dependency.
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::types::InputSource;
use ryo_execution_domain::domain::executable_program::types::RegionId;
use ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram;
use ryo_execution_domain::domain::file_program::types::ObservationSlot;
use ryo_execution_domain::domain::observation_schedule::types::ObservedFrontierRoot;
use ryo_execution_domain::domain::observation_schedule::types::ResolvedObservationBinding;
use ryo_execution_domain::domain::observation_schedule::types::RuleSchedule;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::FrontierPurpose;
use ryo_execution_domain::domain::rule_intent::types::SemanticNeed;
use ryo_execution_domain::domain::trust::types::RelationRead;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::records::types::ObservationIndex;
use ryo_values::domain::records::types::ObservationReadinessLayout;
use std::sync::Arc;

/// How many rules the request holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleTotal(pub usize);
pub(crate) struct RootMetadata {
    pub source: Result<InputSource, OperatingError>,
    pub phase: StepScope,
    pub reads: Box<[RelationRead]>,
}
pub(crate) struct RuleMetadata {
    pub emit: Emit,
    pub emit_root: Result<RegionId, OperatingError>,
    pub frontiers: Box<[Input]>,
    pub observations: Box<[RootMetadata]>,
    pub observation_frontiers: Box<[FrontierPurpose]>,
    pub semantic_needed: SemanticNeed,
    pub frontier_dependencies: Result<
        Vec<Vec<ryo_values::domain::plan_parts::types::ObservationBindingId>>,
        OperatingError,
    >,
}
#[derive(Clone, Copy)]
pub(crate) enum OutputRoot {
    PerFile,
    CrossFile,
    Unavailable,
}
pub(crate) struct OutputMetadata {
    pub root: OutputRoot,
    pub consumers: Vec<RuleIndex>,
}
pub(crate) struct ObservationMetadata {
    pub(crate) layout: ObservationReadinessLayout,
    pub(crate) slots: Box<[ObservationSlot]>,
    pub(crate) frontiers: Box<[ObservedFrontierRoot]>,
    pub(crate) bindings: Vec<Vec<ResolvedObservationBinding>>,
    pub(crate) consumers: Vec<Vec<Vec<RuleIndex>>>,
    pub(crate) outputs: Vec<Vec<OutputMetadata>>,
    pub(crate) retire:
        Vec<Vec<ryo_execution_domain::domain::file_program::types::ObservationRetirement>>,
}
/// Immutable executable state has no RequestGraph, Plan, layout or old executor. Only the
/// preparation owner builds one, and no code outside the engine can name it.
///
/// ```compile_fail,E0603
/// use ryo_execution_runtime::application::file_dag::program::preparation::types::PreparedProgram;
/// ```
pub(crate) struct PreparedProgram {
    pub(super) program: Arc<ExecutableProgram>,
    pub(super) rule_ids: Box<[ryo_values::domain::runs::types::RuleId]>,
    pub(super) applicability: Box<[ryo_values::domain::runs::types::RuleApplicability]>,
    pub(super) rule_digests: Box<[ryo_values::domain::stored_facts::types::Digest]>,
    pub(super) bindings: Box<[Box<[ParameterBinding]>]>,
    pub(super) rules: Box<[RuleMetadata]>,
    /// Computed once when the program is built; never a per-call scan of `rules`.
    pub(super) semantic_needed: SemanticNeed,
    pub(super) file_demands: Box<
        [ryo_execution_domain::domain::executable_program::types::file_rule_demand::value::FileRuleDemand],
    >,
    pub(super) file_frontier_outputs: Box<[Box<[Box<[ObservationIndex]>]>]>,
    pub(super) observations: Option<ObservationMetadata>,
    pub(super) schedule: RuleSchedule,
    pub(super) semantic_metadata: Vec<ryo_values::domain::run_metadata::types::RuleRunMetadata>,
    pub(super) hosted: Box<[HostRelation]>,
    pub(super) matchers:
        Arc<ryo_execution_domain::domain::matching::types::prepared::value::PreparedMatchers>,
    pub(super) derived:
        ryo_execution_store::application::file_dag::derived::prepared::RequestDerived,
    pub(super) walk:
        ryo_execution_domain::domain::shared_walk::types::prepared_walk::value::PreparedWalk,
}
