//! The read-only views of a prepared program.
use crate::application::file_dag::program::prepared_execution::interface::PreparedExecution;
use crate::application::file_dag::program::prepared_execution::interface::PreparedReporting;
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::file_program::types::ObservedFrontierRoot;
use crate::domain::operating::errors::OperatingError;
use crate::domain::trust::types::RelationRead;
use ryo_values::domain::stored_facts::types::Bucket;
use crate::application::file_dag::derived::owner::types::DerivedAccess;
use crate::application::file_dag::execution::CommonOperatorEngine;
use crate::domain::executable_program::types::RoutePolicy;
use crate::domain::prepared_request::types::{Compiled, ObservationIdentity};
use crate::outbound::operators::interface::OperatorsPort;
use ryo_values::domain::{
    hosted::types::HostRelation, rule_requests::types::RuleSpec, stored_facts::types::Digest,
};
impl PreparedExecution for PreparedExecutionProgram {
    fn observation_batches(&self) -> Option<&[Vec<RuleIndex>]> {
        self.inner.observation_batches()
    }
    fn rule_count(&self) -> usize {
        self.inner.rule_count().0
    }
    fn rule_order(&self) -> impl Iterator<Item = RuleIndex> + '_ {
        self.inner.rule_order()
    }
    fn demanded_rules(&self, governs: impl Fn(usize) -> bool) -> Vec<bool> {
        use crate::application::file_dag::program::preparation::types::Demand;
        self.inner
            .demanded_rules(|rule| {
                if governs(rule.0) {
                    Demand::Demanded
                } else {
                    Demand::Undemanded
                }
            })
            .into_iter()
            .map(|demand| demand == Demand::Demanded)
            .collect()
    }
    fn emit(
        &self,
        rule: RuleIndex,
    ) -> Result<&ryo_values::domain::plans::types::Emit, OperatingError> {
        self.inner.emit(rule)
    }
    fn rule_digest(
        &self,
        rule: RuleIndex,
    ) -> Result<ryo_values::domain::stored_facts::types::Digest, OperatingError> {
        self.inner.rule_digest(rule)
    }
    fn frontiers(
        &self,
        rule: RuleIndex,
    ) -> Result<&[ryo_values::domain::plan_parts::types::Input], OperatingError> {
        self.inner.frontiers(rule)
    }
    fn captures_observation(&self, rule: RuleIndex, input: usize) -> Result<bool, OperatingError> {
        Ok(self
            .inner
            .frontier_purpose(rule, ryo_values::domain::plan_parts::types::InputNumber(input))?
            == crate::domain::executable_program::file_demand::types::FrontierPurpose::Observation)
    }
    fn semantic_needed(&self) -> bool {
        self.inner.semantic_needed() == crate::application::file_dag::program::preparation::types::SemanticNeed::Required
    }
    fn observation_layout(
        &self,
    ) -> Option<(
        ryo_values::domain::records::types::ObservationReadinessLayout,
        &[ObservedFrontierRoot],
    )> {
        self.inner
            .observation_layout()
            .map(|named| (named.layout, named.frontiers))
    }
    fn observation_reads(&self, rule: RuleIndex) -> Result<Vec<RelationRead>, OperatingError> {
        self.inner.observation_reads(rule)
    }
    fn has_observation_contract(&self, rule: RuleIndex) -> Result<bool, OperatingError> {
        Ok(self.inner.has_observation_contract(rule)?
            == crate::application::file_dag::program::preparation::types::ObservationContract::Declared)
    }
    fn frontier_dependencies(
        &self,
        rule: RuleIndex,
    ) -> Result<&[Vec<ryo_values::domain::plan_parts::types::ObservationBindingId>], OperatingError>
    {
        self.inner.frontier_dependencies(rule)
    }
}
impl PreparedReporting for PreparedExecutionProgram {
    fn metadata(&self) -> &[ryo_values::domain::run_metadata::types::RuleRunMetadata] {
        self.inner.metadata()
    }
    fn replay_closure(
        &self,
        again: ryo_values::domain::stores::types::Reuse,
        first: &[Bucket],
    ) -> Result<(ryo_values::domain::stores::types::Reuse, Vec<Bucket>), OperatingError> {
        let closure = self.inner.replay_closure(again, first)?;
        Ok((closure.reuse, closure.buckets))
    }
}

/// The program of a request prepared over the engine's operators and derived views.
pub(crate) fn prepare(
    compiled: Compiled,
    rules: &[RuleSpec],
    digests: &[Digest],
    hosted: &[HostRelation],
    identity: ObservationIdentity,
    operators: Arc<dyn OperatorsPort>,
    derived: &DerivedAccess,
    routes: RoutePolicy,
) -> Result<PreparedExecutionProgram, OperatingError> {
    let inner = crate::application::file_dag::program::preparation::service::prepare(
        compiled,
        rules,
        digests,
        hosted,
        derived.clone(),
        identity,
        routes,
    )?;
    Ok(PreparedExecutionProgram {
        inner,
        kernels: Arc::new(CommonOperatorEngine::new(operators)),
        derived: derived.clone(),
    })
}
