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

/// How many rules the request holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleTotal(pub usize);
/// One named output of a rule as its plan states it: where it runs and what it reads on the way.
pub(crate) struct RootPlan {
    pub phase: StepScope,
    pub reads: Box<[RelationRead]>,
}
/// What a rule's plan states, read once from the plans and the file pass's program, never from the interpreter's compile.
pub(crate) struct RulePlan {
    pub emit: Emit,
    pub frontiers: Box<[Input]>,
    pub roots: Box<[RootPlan]>,
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
/// The request's named outputs as the plans and the observation request state them: the readiness layout, each output a
/// frontier carries, each rule's bindings and their admissions, each output's root and consumers, and what each consumer
/// retires.
pub(crate) struct ObservationMetadata {
    pub(crate) layout: ObservationReadinessLayout,
    pub(crate) frontiers: Box<[ObservedFrontierRoot]>,
    pub(crate) bindings: Vec<Vec<ResolvedObservationBinding>>,
    /// Each binding of each rule checked against its consumer's declaration and its producers' outputs, in binding order: a
    /// binding refused here is withheld with its refusal wherever a step reads it.
    pub(crate) admissions: Vec<Vec<Result<(), OperatingError>>>,
    pub(crate) outputs: Vec<Vec<OutputMetadata>>,
    pub(crate) retire:
        Vec<Vec<ryo_execution_domain::domain::observation_schedule::types::ObservationRetirement>>,
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
    /// Each rule's plan, in rule order.
    pub(super) rules: Box<[RulePlan]>,
    /// Computed once when the program is built; never a per-call scan of `rules`.
    pub(super) semantic_needed: SemanticNeed,
    pub(super) file_frontier_outputs: Box<[Box<[Box<[ObservationIndex]>]>]>,
    /// The named outputs, when the request names any.
    pub(super) observed: Option<ObservationMetadata>,
    pub(super) schedule: RuleSchedule,
    pub(super) semantic_metadata: Vec<ryo_values::domain::run_metadata::types::RuleRunMetadata>,
    pub(super) hosted: Box<[HostRelation]>,
    /// The relations the request's plans read, each once, in the order first read: what each file's sources are asked for.
    pub(super) demanded: Box<[ryo_relations::domain::relations::types::Relation]>,
}
