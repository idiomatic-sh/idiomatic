//! The read-only views of a prepared program.
use std::sync::Arc;
use crate::application::file_dag::program::prepared_execution::interface::PreparedExecution;
use crate::application::file_dag::program::prepared_execution::interface::PreparedReporting;
use crate::application::file_dag::program::prepared_execution::types::{PreparedExecutionProgram, ProgramParts};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use crate::application::file_dag::program::preparation::types::{
    Demand, NamedLayout, ObservationContract, ReplayClosure, RuleTotal, SemanticNeed,
};
use ryo_execution_domain::domain::rule_intent::types::FrontierPurpose;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::trust::types::RelationRead;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_execution_store::application::file_dag::derived::owner::types::DerivedAccess;
use crate::application::file_dag::execution::CommonOperatorEngine;
use ryo_execution_domain::domain::prepared_request::types::{Compiled, ObservationIdentity};
use crate::outbound::operators::interface::OperatorsPort;
use ryo_values::domain::{
    hosted::types::HostRelation, rule_requests::types::RuleSpec, stored_facts::types::Digest,
};
impl PreparedExecution for PreparedExecutionProgram {
    fn observation_batches(&self) -> Option<&[Vec<RuleIndex>]> {
        self.parts.inner.observation_batches()
    }
    fn rule_count(&self) -> RuleTotal {
        self.parts.inner.rule_count()
    }
    fn rule_order(&self) -> impl Iterator<Item = RuleIndex> + '_ {
        self.parts.inner.rule_order()
    }
    fn demanded_rules(&self, governs: impl Fn(RuleIndex) -> Demand) -> Vec<Demand> {
        self.parts.inner.demanded_rules(governs)
    }
    fn emit(
        &self,
        rule: RuleIndex,
    ) -> Result<&ryo_values::domain::plans::types::Emit, OperatingError> {
        self.parts.inner.emit(rule)
    }
    fn rule_digest(
        &self,
        rule: RuleIndex,
    ) -> Result<ryo_values::domain::stored_facts::types::Digest, OperatingError> {
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
    fn observation_layout(&self) -> Option<NamedLayout<'_>> {
        self.parts.inner.observation_layout()
    }
    fn observation_reads(&self, rule: RuleIndex) -> Result<Vec<RelationRead>, OperatingError> {
        self.parts.inner.observation_reads(rule)
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
    ) -> Result<&[Vec<ryo_values::domain::plan_parts::types::ObservationBindingId>], OperatingError>
    {
        self.parts.inner.frontier_dependencies(rule)
    }
}
/// The prepared program's matchers, for the execution crate's use-case proofs of lazy matcher compilation.
#[cfg(any(test, feature = "fixtures"))]
impl PreparedExecutionProgram {
    /// The matchers the program prepared, compiled on first use.
    #[must_use]
    pub fn matchers(&self) -> &Arc<ryo_execution_domain::domain::matching::types::prepared::value::PreparedMatchers> {
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

/// The program of a request prepared over the engine's operators and derived views.
pub fn prepare(
    compiled: Compiled,
    rules: &[RuleSpec],
    digests: &[Digest],
    hosted: &[HostRelation],
    identity: ObservationIdentity,
    operators: Arc<dyn OperatorsPort>,
    derived: &DerivedAccess,
    digest: Arc<dyn crate::outbound::digest::interface::DigestPort>,
) -> Result<PreparedExecutionProgram, OperatingError> {
    let inner = crate::application::file_dag::program::preparation::service::prepare(
        compiled,
        rules,
        digests,
        hosted,
        derived.clone(),
        identity,
    )?;
    Ok(PreparedExecutionProgram {
        parts: ProgramParts {
            inner,
            kernels: Arc::new(CommonOperatorEngine::new(operators)),
            derived: derived.clone(),
            digest,
        },
    })
}
