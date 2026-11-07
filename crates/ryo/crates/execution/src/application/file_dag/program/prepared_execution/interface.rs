//! What a caller may read of a prepared program.
use crate::domain::coverage::types::RuleIndex;
use crate::domain::file_program::types::ObservedFrontierRoot;
use crate::domain::operating::errors::OperatingError;
use crate::domain::trust::types::RelationRead;
use ryo_values::domain::stored_facts::types::Bucket;
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
pub trait PreparedReporting {
    fn metadata(&self) -> &[ryo_values::domain::run_metadata::types::RuleRunMetadata];
    fn replay_closure(
        &self,
        again: ryo_values::domain::stores::types::Reuse,
        first: &[Bucket],
    ) -> Result<(ryo_values::domain::stores::types::Reuse, Vec<Bucket>), OperatingError>;
}
