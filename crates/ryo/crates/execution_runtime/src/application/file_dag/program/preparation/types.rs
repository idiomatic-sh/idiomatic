//! Immutable prepared request metadata; no runtime or lifecycle dependency.
use ryo_execution_domain::domain::coverage::types::RuleIndex;
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
use ryo_values::domain::records::types::ObservationIndex;
use ryo_values::domain::records::types::ObservationReadinessLayout;
use std::sync::Arc;

/// How many rules the request holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleTotal(pub usize);
pub(crate) struct RootMetadata {
    pub phase: StepScope,
    pub reads: Box<[RelationRead]>,
}
pub(crate) struct RuleMetadata {
    pub emit: Emit,
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
    pub(crate) frontiers: Box<[ObservedFrontierRoot]>,
    pub(crate) bindings: Vec<Vec<ResolvedObservationBinding>>,
    /// Each binding of each rule checked against its consumer's declaration and its producers' outputs, in binding order: a
    /// binding refused here is withheld with its refusal wherever a step reads it.
    pub(crate) admissions: Vec<Vec<Result<(), OperatingError>>>,
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
    pub(super) rule_ids: Box<[ryo_values::domain::runs::types::RuleId]>,
    pub(super) applicability: Box<[ryo_values::domain::runs::types::RuleApplicability]>,
    pub(super) rule_digests: Box<[ryo_values::domain::stored_facts::types::Digest]>,
    pub(super) rules: Box<[RuleMetadata]>,
    /// Computed once when the program is built; never a per-call scan of `rules`.
    pub(super) semantic_needed: SemanticNeed,
    pub(super) file_frontier_outputs: Box<[Box<[Box<[ObservationIndex]>]>]>,
    pub(super) observations: Option<ObservationMetadata>,
    pub(super) schedule: RuleSchedule,
    pub(super) semantic_metadata: Vec<ryo_values::domain::run_metadata::types::RuleRunMetadata>,
    pub(super) hosted: Box<[HostRelation]>,
    pub(super) matchers:
        Arc<ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers>,
    /// The relations the request's plans read, each once, in the order first read: what each file's sources are asked for.
    pub(super) demanded: Box<[ryo_relations::domain::relations::types::Relation]>,
}
