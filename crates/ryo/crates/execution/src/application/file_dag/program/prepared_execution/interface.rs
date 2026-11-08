//! What a caller may read of a prepared program.
use crate::application::file_dag::program::preparation::types::{
    Demand, NamedLayout, ObservationContract, ReplayClosure, RuleTotal, SemanticNeed,
};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::rule_intent::types::FrontierPurpose;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::trust::types::RelationRead;
use ryo_values::domain::stored_facts::types::Bucket;
/// Read-only request metadata; no mutable program, permit or population escapes.
pub trait PreparedExecution {
    fn observation_batches(&self) -> Option<&[Vec<RuleIndex>]>;
    fn rule_count(&self) -> RuleTotal;
    fn rule_order(&self) -> impl Iterator<Item = RuleIndex> + '_;
    fn demanded_rules(&self, governs: impl Fn(RuleIndex) -> Demand) -> Vec<Demand>;
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
    fn frontier_purpose(
        &self,
        rule: RuleIndex,
        input: ryo_values::domain::plan_parts::types::InputNumber,
    ) -> Result<FrontierPurpose, OperatingError>;
    fn semantic_needed(&self) -> SemanticNeed;
    fn observation_layout(&self) -> Option<NamedLayout<'_>>;
    fn observation_reads(&self, rule: RuleIndex) -> Result<Vec<RelationRead>, OperatingError>;
    fn has_observation_contract(
        &self,
        rule: RuleIndex,
    ) -> Result<ObservationContract, OperatingError>;
    fn frontier_dependencies(
        &self,
        rule: RuleIndex,
    ) -> Result<&[Vec<ryo_values::domain::plan_parts::types::ObservationBindingId>], OperatingError>;
}
pub trait PreparedReporting {
    fn metadata(&self) -> &[ryo_values::domain::run_metadata::types::RuleRunMetadata];
    fn replay_closure(
        &self,
        again: ryo_values::domain::stores::types::Reuse,
        first: &[Bucket],
    ) -> Result<ReplayClosure, OperatingError>;
}
