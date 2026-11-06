//! Immutable prepared request metadata; no runtime or lifecycle dependency.
use crate::domain::executable_program::types::ExecutableProgram;
use crate::domain::executable_program::types::InputSource;
use crate::domain::executable_program::types::RegionId;
use crate::domain::file_program::types::ObservationSlot;
use crate::domain::file_program::types::ObservedFrontierRoot;
use crate::domain::file_program::types::ResolvedObservationBinding;
use crate::domain::file_program::types::RuleSchedule;
use crate::domain::operating::errors::OperatingError;
use crate::domain::trust::types::RelationRead;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::observations::types::ObservationReadinessLayout;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::plans::types::StepScope;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use std::sync::Arc;

pub(in crate::application::file_dag::program) struct RootMetadata {
    pub source: Result<InputSource, OperatingError>,
    pub phase: StepScope,
    pub reads: Box<[RelationRead]>,
}
pub(in crate::application::file_dag::program) struct RuleMetadata {
    pub emit: Emit,
    pub emit_root: Result<RegionId, OperatingError>,
    pub frontiers: Box<[Input]>,
    pub observations: Box<[RootMetadata]>,
    pub observation_frontiers: Box<[bool]>,
    pub semantic_needed: bool,
    pub frontier_dependencies: Result<
        Vec<Vec<ryo_values::domain::observations::types::ObservationBindingId>>,
        OperatingError,
    >,
}
#[derive(Clone, Copy)]
pub(in crate::application) enum OutputRoot {
    PerFile,
    CrossFile,
    Unavailable,
}
pub(in crate::application) struct OutputMetadata {
    pub root: OutputRoot,
    pub consumers: Vec<usize>,
}
pub(crate) struct ObservationMetadata {
    pub(in crate::application) layout: ObservationReadinessLayout,
    pub(in crate::application) slots: Box<[ObservationSlot]>,
    pub(in crate::application) frontiers: Box<[ObservedFrontierRoot]>,
    pub(in crate::application) bindings: Vec<Vec<ResolvedObservationBinding>>,
    pub(in crate::application) consumers: Vec<Vec<Vec<usize>>>,
    pub(in crate::application) outputs: Vec<Vec<OutputMetadata>>,
    pub(in crate::application) retire:
        Vec<Vec<crate::domain::file_program::types::ObservationRetirement>>,
}
/// Immutable executable state has no RequestGraph, Plan, layout or old executor.
pub(crate) struct PreparedProgram {
    pub(super) program: Arc<ExecutableProgram>,
    pub(super) rule_ids: Box<[ryo_values::domain::runs::types::RuleId]>,
    pub(super) applicability: Box<[ryo_values::domain::runs::types::RuleApplicability]>,
    pub(super) rule_digests: Box<[ryo_values::domain::stored_facts::types::Digest]>,
    pub(super) bindings: Box<[Box<[ParameterBinding]>]>,
    pub(super) rules: Box<[RuleMetadata]>,
    pub(super) file_demands:
        Box<[crate::domain::executable_program::file_demand::types::FileRuleDemand]>,
    pub(super) file_frontier_outputs: Box<[Box<[Box<[usize]>]>]>,
    pub(super) observations: Option<ObservationMetadata>,
    pub(super) schedule: RuleSchedule,
    pub(super) semantic_metadata: Vec<ryo_values::domain::run_metadata::types::RuleMetadata>,
    pub(super) hosted: Box<[HostRelation]>,
    pub(super) matchers: Arc<crate::domain::matching::types::prepared::value::PreparedMatchers>,
    pub(super) derived: crate::application::file_dag::derived::prepared::PreparedDerived,
    pub(super) native_orders:
        Box<[Option<Arc<ryo_derived::domain::contract::types::PreparedDerived>>]>,
    pub(super) walk: crate::domain::shared_walk::types::prepared_walk::value::PreparedWalk,
}
