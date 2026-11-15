//! Immutable prepared request metadata; no runtime or lifecycle dependency.
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::file_program::types::ObservationSlot;
use ryo_execution_domain::domain::interpreted_program::types::InputSource;
use ryo_execution_domain::domain::interpreted_program::types::RegionId;
use ryo_execution_domain::domain::interpreted_program::types::interpreted_program::value::InterpretedProgram;
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
/// The request's named outputs as the plans and the observation request state them: the readiness layout and each output a
/// frontier carries.
pub(crate) struct ObservedLayout {
    pub(crate) layout: ObservationReadinessLayout,
    pub(crate) frontiers: Box<[ObservedFrontierRoot]>,
    pub(crate) bindings: Vec<Vec<ResolvedObservationBinding>>,
    pub(crate) consumers: Vec<Vec<Vec<RuleIndex>>>,
}
/// What the interpreter compiled for the project phase, which the fourth stage deletes: its program, matchers and walk, each
/// rule's regions and the named outputs' graph.
pub(crate) struct Interpreted {
    pub(crate) program: Arc<InterpretedProgram>,
    pub(crate) rules: Box<[RuleMetadata]>,
    pub(crate) observations: Option<ObservationMetadata>,
    pub(crate) matchers:
        Arc<ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers>,
    pub(crate) walk:
        ryo_execution_domain::domain::shared_walk::types::prepared_walk::value::PreparedWalk,
}
/// A rule's interpreter regions, for the project phase: its emitted region and each named output's source and reads.
pub(crate) struct RuleMetadata {
    pub emit_root: Result<RegionId, OperatingError>,
    pub observations: Box<[RootMetadata]>,
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
    pub(super) bindings: Box<[Box<[ParameterBinding]>]>,
    /// Each rule's plan, in rule order.
    pub(super) rules: Box<[RulePlan]>,
    /// Computed once when the program is built; never a per-call scan of `rules`.
    pub(super) semantic_needed: SemanticNeed,
    pub(super) file_frontier_outputs: Box<[Box<[Box<[ObservationIndex]>]>]>,
    /// The named outputs, when the request names any.
    pub(super) observed: Option<ObservedLayout>,
    pub(super) schedule: RuleSchedule,
    pub(super) semantic_metadata: Vec<ryo_values::domain::run_metadata::types::RuleRunMetadata>,
    pub(super) hosted: Box<[HostRelation]>,
    /// The relations the request's plans read, each once, in the order first read: what each file's sources are asked for.
    pub(super) demanded: Box<[ryo_relations::domain::relations::types::Relation]>,
    /// The interpreter's compile, for the project phase alone, or why it could not be made: it never fails the request.
    pub(super) interpreted: Result<Interpreted, OperatingError>,
}
/// What the interpreter's compile of a request reads: the plans, owned, what they bind, the rules, the hosted relations, the
/// derived views, the observation request, each rule's plan and the file pass's program, whose refusals it takes.
pub(crate) struct Interpreting<'a> {
    pub(crate) plans: Vec<ryo_values::domain::plans::types::Plan>,
    pub(crate) bindings: &'a [Vec<ParameterBinding>],
    pub(crate) rules: &'a [ryo_values::domain::rule_requests::types::RuleSpec],
    pub(crate) hosted: &'a [HostRelation],
    pub(crate) derived: crate::application::file_dag::derived::program::service::ProgramDerived,
    pub(crate) observations: &'a [Vec<ResolvedObservationBinding>],
    pub(crate) request: ryo_execution_domain::domain::observation_schedule::types::ObservationRequest<'a>,
    pub(crate) rule_plans: &'a [RulePlan],
    pub(crate) executable:
        &'a ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram,
    pub(crate) partitions: ryo_execution_domain::domain::source_partitions::types::source_partitions::value::SourcePartitions,
}
