//! What a caller may read of a prepared program.
use crate::application::file_dag::program::preparation::types::RuleTotal;
use crate::application::file_dag::program::prepared_execution::types::{
    Demand, NamedLayout, ObservationContract, ReplayClosure,
};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::FrontierPurpose;
use ryo_execution_domain::domain::rule_intent::types::SemanticNeed;
use ryo_execution_domain::domain::trust::types::RelationRead;
use ryo_values::domain::stored_facts::types::Bucket;
/// Read-only request metadata; no mutable program, permit or population escapes.
pub trait PreparedExecution {
    fn observation_batches(&self) -> Option<&[Vec<RuleIndex>]>;
    fn rule_count(&self) -> RuleTotal;
    fn rule_order(&self) -> impl Iterator<Item = RuleIndex> + '_;
    fn demanded_rules(&self, governs: impl Fn(RuleIndex) -> Demand) -> Vec<Demand>;
    /// The rule's emitted root.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] for a rule the program does not hold.
    fn emit(
        &self,
        rule: RuleIndex,
    ) -> Result<&ryo_values::domain::plans::types::Emit, OperatingError>;
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
    fn semantic_needed(&self) -> SemanticNeed;
    fn observation_layout(&self) -> Option<NamedLayout<'_>>;
    /// The relations the rule's observations read.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] for a rule the program does not hold.
    fn observation_reads(&self, rule: RuleIndex) -> Result<Vec<RelationRead>, OperatingError>;
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
    ) -> Result<&[Vec<ryo_values::domain::plan_parts::types::ObservationBindingId>], OperatingError>;
}
pub trait PreparedReporting {
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
