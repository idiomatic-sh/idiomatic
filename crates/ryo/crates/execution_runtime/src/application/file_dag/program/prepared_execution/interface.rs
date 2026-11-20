//! What a caller may read of a prepared program.
use crate::application::file_dag::program::preparation::types::RuleTotal;
use crate::application::file_dag::program::prepared_execution::types::{
    Demand, Demands, FrontierDependencies, ObservationContract, ObservationLayout, RelationReads,
    ReplayClosure, RuleBatches, RuleOrder,
};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::FrontierPurpose;
use ryo_execution_domain::domain::rule_intent::types::SemanticNeed;
use ryo_values::domain::stored_facts::types::Bucket;
/// Read-only request metadata; no mutable program, permit or population escapes.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::program::prepared_execution::interface::ProgramPreparedExecutionPort;
/// fn prepared(program: &impl ProgramPreparedExecutionPort) {
///     let _ = program;
/// }
/// ```
pub trait ProgramPreparedExecutionPort {
    /// The program's rules grouped into the batches in which their observations become ready.
    fn observation_batches(&self) -> RuleBatches<'_>;
    /// How many rules the program holds.
    fn rule_count(&self) -> RuleTotal;
    /// The order the program runs its rules in.
    fn rule_order(&self) -> RuleOrder<'_>;
    /// Which rules the request wants, given which rules govern a file: `governs` answers for each rule.
    fn demanded_rules(&self, governs: impl Fn(RuleIndex) -> Demand) -> Demands;
    /// The rule's emitted root.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] for a rule the program does not hold.
    fn emit(
        &self,
        rule: RuleIndex,
    ) -> Result<&ryo_values::domain::query_plans::types::Emit, OperatingError>;
    /// The rule's digest.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] for a rule the program does not hold.
    fn rule_digest(
        &self,
        rule: RuleIndex,
    ) -> Result<ryo_values::domain::stored_facts::types::Digest, OperatingError>;
    /// The rule's cross-file frontier inputs.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] for a rule the program does not hold.
    fn frontiers(
        &self,
        rule: RuleIndex,
    ) -> Result<&[ryo_values::domain::plan_parts::types::Input], OperatingError>;
    /// What one frontier input of the rule is for.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] for a rule the program does not hold or an input it has no frontier at.
    fn frontier_purpose(
        &self,
        rule: RuleIndex,
        input: ryo_values::domain::plan_parts::types::InputNumber,
    ) -> Result<FrontierPurpose, OperatingError>;
    /// Whether the program's rules need a semantic session for the files they run over.
    fn semantic_needed(&self) -> SemanticNeed;
    /// The readiness layout of the program's named observations, when it has any.
    fn observation_layout(&self) -> ObservationLayout<'_>;
    /// The relations the rule's observations read.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] for a rule the program does not hold.
    fn observation_reads(&self, rule: RuleIndex) -> Result<RelationReads, OperatingError>;
    /// Whether the rule declares or binds an observation.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] for a rule the program does not hold.
    fn has_observation_contract(
        &self,
        rule: RuleIndex,
    ) -> Result<ObservationContract, OperatingError>;
    /// The bound observations each of the rule's frontiers depends on.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] for a rule the program does not hold, and the refusal its frontier dependencies were
    /// prepared with.
    fn frontier_dependencies(
        &self,
        rule: RuleIndex,
    ) -> Result<FrontierDependencies<'_>, OperatingError>;
}
/// What a prepared program reports of itself: its run metadata and the rules a changed selection makes run again.
pub trait PreparedReporting {
    /// The run metadata of each rule, in rule order.
    fn metadata(&self) -> &[ryo_values::domain::run_metadata::types::RuleRunMetadata];
    /// The rules a replay must run again, from the first buckets of a run.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] for a rule or observation the program does not hold.
    fn replay_closure(
        &self,
        again: ryo_values::domain::stores::types::Reuse,
        first: &[Bucket],
    ) -> Result<ReplayClosure, OperatingError>;
}
