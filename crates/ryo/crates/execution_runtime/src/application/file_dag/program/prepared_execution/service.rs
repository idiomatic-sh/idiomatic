//! The read-only views of a prepared program.
use crate::application::file_dag::program::preparation::types::RuleTotal;
use crate::application::file_dag::program::prepared_execution::interface::PreparedReporting;
use crate::application::file_dag::program::prepared_execution::interface::ProgramPreparedExecutionPort;
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use crate::application::file_dag::program::prepared_execution::types::{
    Demand, ObservationContract, ReplayClosure,
};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::FrontierPurpose;
use ryo_execution_domain::domain::rule_intent::types::SemanticNeed;
use ryo_values::domain::stored_facts::types::{Bucket, Digest};
impl ProgramPreparedExecutionPort for PreparedExecutionProgram {
    fn observation_batches(
        &self,
    ) -> crate::application::file_dag::program::prepared_execution::types::RuleBatches<'_> {
        crate::application::file_dag::program::prepared_execution::types::RuleBatches(
            self.parts.inner.observation_batches(),
        )
    }
    fn rule_count(&self) -> RuleTotal {
        self.parts.inner.rule_count()
    }
    fn rule_order(
        &self,
    ) -> crate::application::file_dag::program::prepared_execution::types::RuleOrder<'_> {
        self.parts.inner.rule_order()
    }
    fn demanded_rules(
        &self,
        governs: impl Fn(RuleIndex) -> Demand,
    ) -> crate::application::file_dag::program::prepared_execution::types::Demands {
        crate::application::file_dag::program::prepared_execution::types::Demands(
            self.parts.inner.demanded_rules(governs),
        )
    }
    fn emit(
        &self,
        rule: RuleIndex,
    ) -> Result<&ryo_values::domain::plans::types::Emit, OperatingError> {
        self.parts.inner.emit(rule)
    }
    fn rule_digest(&self, rule: RuleIndex) -> Result<Digest, OperatingError> {
        self.parts.inner.rule_digest(rule)
    }
    fn frontiers(
        &self,
        rule: RuleIndex,
    ) -> Result<&[ryo_values::domain::plan_parts::types::Input], OperatingError> {
        self.parts.inner.frontiers(rule)
    }
    fn frontier_purpose(
        &self,
        rule: RuleIndex,
        input: ryo_values::domain::plan_parts::types::InputNumber,
    ) -> Result<FrontierPurpose, OperatingError> {
        self.parts.inner.frontier_purpose(rule, input)
    }
    fn semantic_needed(&self) -> SemanticNeed {
        self.parts.inner.semantic_needed()
    }
    fn observation_layout(
        &self,
    ) -> crate::application::file_dag::program::prepared_execution::types::ObservationLayout<'_>
    {
        crate::application::file_dag::program::prepared_execution::types::ObservationLayout(
            self.parts.inner.observation_layout(),
        )
    }
    fn observation_reads(
        &self,
        rule: RuleIndex,
    ) -> Result<
        crate::application::file_dag::program::prepared_execution::types::RelationReads,
        OperatingError,
    > {
        self.parts
            .inner
            .observation_reads(rule)
            .map(crate::application::file_dag::program::prepared_execution::types::RelationReads)
    }
    fn has_observation_contract(
        &self,
        rule: RuleIndex,
    ) -> Result<ObservationContract, OperatingError> {
        self.parts.inner.has_observation_contract(rule)
    }
    fn frontier_dependencies(
        &self,
        rule: RuleIndex,
    ) -> Result<
        crate::application::file_dag::program::prepared_execution::types::FrontierDependencies<'_>,
        OperatingError,
    > {
        self.parts.inner.frontier_dependencies(rule).map(
            crate::application::file_dag::program::prepared_execution::types::FrontierDependencies,
        )
    }
}
/// The prepared program's matchers, for the execution crate's use-case proofs of lazy matcher compilation.
#[cfg(any(test, feature = "fixtures"))]
impl PreparedExecutionProgram {
    /// The matchers the program prepared, compiled on first use.
    #[must_use]
    pub fn matchers(
        &self,
    ) -> &std::sync::Arc<ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers>
    {
        self.parts.inner.matchers()
    }
}

impl PreparedReporting for PreparedExecutionProgram {
    fn metadata(&self) -> &[ryo_values::domain::run_metadata::types::RuleRunMetadata] {
        self.parts.inner.metadata()
    }
    fn replay_closure(
        &self,
        again: ryo_values::domain::stores::types::Reuse,
        first: &[Bucket],
    ) -> Result<ReplayClosure, OperatingError> {
        self.parts.inner.replay_closure(again, first)
    }
}
