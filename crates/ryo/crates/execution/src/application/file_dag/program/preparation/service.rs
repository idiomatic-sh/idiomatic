//! Immutable request preparation, physical metadata and compilation ownership.
use super::types::*;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::rule_intent::types::FrontierPurpose;
use crate::domain::executable_program::types::ExecutableProgram;
use crate::domain::operating::errors::OperatingError;
use crate::domain::prepared_request::types::Compiled;
use crate::domain::prepared_request::types::ObservationIdentity;
use crate::domain::source_partitions::types::source_partitions::value::SourcePartitions;
use crate::domain::trust::types::RelationRead;

use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::records::types::ObservationIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rule_requests::types::RuleSpec;
use std::sync::Arc;

mod census;
mod demands;
mod metadata;
mod orders;
mod replay;
mod schedule;

impl PreparedProgram {
    pub(in crate::application::file_dag::program) fn file_demand(
        &self,
        rule: RuleIndex,
    ) -> Result<
        &crate::domain::executable_program::file_demand::types::FileRuleDemand,
        OperatingError,
    > {
        self.file_demands.get(rule.0).ok_or(OperatingError::NotRun)
    }
    pub(in crate::application::file_dag::program) fn file_frontier_outputs(
        &self,
        rule: RuleIndex,
        input: InputNumber,
    ) -> Result<&[ObservationIndex], OperatingError> {
        self.file_frontier_outputs
            .get(rule.0)
            .and_then(|inputs| inputs.get(input.0))
            .map(AsRef::as_ref)
            .ok_or(OperatingError::NotRun)
    }
    pub(in crate::application::file_dag::program) fn native_order(
        &self,
        region: crate::domain::executable_program::types::RegionId,
    ) -> Result<Option<Arc<crate::domain::derived_views::types::DerivedContract>>, OperatingError>
    {
        self.native_orders
            .get(region.0)
            .cloned()
            .ok_or(OperatingError::NotRun)
    }

    pub(in crate::application::file_dag::program) fn walk(
        &self,
    ) -> &crate::domain::shared_walk::types::prepared_walk::value::PreparedWalk {
        &self.walk
    }
    pub(in crate::application::file_dag::program) fn all_bindings(
        &self,
    ) -> &[Box<[ParameterBinding]>] {
        &self.bindings
    }
    pub(in crate::application::file_dag::program) fn walk_class(
        &self,
        instruction: &crate::domain::executable_program::types::Instruction,
    ) -> Option<crate::domain::positions::types::ClassPlace> {
        self.walk.class_for(instruction)
    }
    pub(in crate::application::file_dag::program) fn rule_id(
        &self,
        rule: RuleIndex,
    ) -> Result<&ryo_values::domain::runs::types::RuleId, OperatingError> {
        self.rule_ids.get(rule.0).ok_or(OperatingError::NotRun)
    }
    pub(in crate::application::file_dag::program) fn applicability(
        &self,
        rule: RuleIndex,
    ) -> Result<ryo_values::domain::runs::types::RuleApplicability, OperatingError> {
        self.applicability
            .get(rule.0)
            .copied()
            .ok_or(OperatingError::NotRun)
    }
    pub(crate) fn rule_digest(
        &self,
        rule: RuleIndex,
    ) -> Result<ryo_values::domain::stored_facts::types::Digest, OperatingError> {
        self.rule_digests
            .get(rule.0)
            .copied()
            .ok_or(OperatingError::NotRun)
    }
    pub(in crate::application::file_dag) fn program(&self) -> &Arc<ExecutableProgram> {
        &self.program
    }
    pub(in crate::application::file_dag::program) fn derived(
        &self,
    ) -> &crate::application::file_dag::derived::prepared::RequestDerived {
        &self.derived
    }
    pub(in crate::application::file_dag::program) fn hosted(&self) -> &[HostRelation] {
        &self.hosted
    }
    pub(crate) fn matchers(
        &self,
    ) -> &Arc<crate::domain::matching::types::prepared::value::PreparedMatchers> {
        &self.matchers
    }
    pub(crate) fn bindings(&self, rule: RuleIndex) -> Result<&[ParameterBinding], OperatingError> {
        self.bindings
            .get(rule.0)
            .map(Box::as_ref)
            .ok_or(OperatingError::NotRun)
    }
    pub(in crate::application::file_dag::program) fn rule_metadata(
        &self,
        rule: RuleIndex,
    ) -> Result<&RuleMetadata, OperatingError> {
        tracing::trace!(
            target: crate::application::file_dag::derived::constants::TARGET,
            event = crate::application::file_dag::derived::constants::RULE_METADATA,
            rule = rule.0
        );
        self.rules.get(rule.0).ok_or(OperatingError::NotRun)
    }
    pub(crate) fn observation_metadata(&self) -> Option<&ObservationMetadata> {
        self.observations.as_ref()
    }
    pub(crate) fn emit(&self, rule: RuleIndex) -> Result<&Emit, OperatingError> {
        Ok(&self.rule_metadata(rule)?.emit)
    }
    pub(crate) fn observation_layout(&self) -> Option<NamedLayout<'_>> {
        self.observations.as_ref().map(|observations| NamedLayout {
            layout: observations.layout,
            frontiers: observations.frontiers.as_ref(),
        })
    }
    pub(crate) fn frontier_purpose(
        &self,
        rule: RuleIndex,
        input: InputNumber,
    ) -> Result<FrontierPurpose, OperatingError> {
        self.rule_metadata(rule)?
            .observation_frontiers
            .get(input.0)
            .copied()
            .ok_or(OperatingError::NotRun)
    }
    pub(crate) fn frontiers(&self, rule: RuleIndex) -> Result<&[Input], OperatingError> {
        Ok(&self.rule_metadata(rule)?.frontiers)
    }
    pub(crate) fn frontier_dependencies(
        &self,
        rule: RuleIndex,
    ) -> Result<&[Vec<ryo_values::domain::plan_parts::types::ObservationBindingId>], OperatingError>
    {
        self.rule_metadata(rule)?
            .frontier_dependencies
            .as_deref()
            .map_err(Clone::clone)
    }
    pub(crate) fn has_observation_contract(
        &self,
        rule: RuleIndex,
    ) -> Result<ObservationContract, OperatingError> {
        let declared = !self.rule_metadata(rule)?.observations.is_empty()
            || self
                .observations
                .as_ref()
                .is_some_and(|named| !named.bindings[rule.0].is_empty());
        Ok(if declared {
            ObservationContract::Declared
        } else {
            ObservationContract::Absent
        })
    }
    pub(crate) fn observation_reads(
        &self,
        rule: RuleIndex,
    ) -> Result<Vec<RelationRead>, OperatingError> {
        Ok(self
            .rule_metadata(rule)?
            .observations
            .iter()
            .flat_map(|root| root.reads.iter().cloned())
            .collect())
    }
    pub(crate) fn metadata(&self) -> &[ryo_values::domain::run_metadata::types::RuleRunMetadata] {
        &self.semantic_metadata
    }

    pub(crate) fn semantic_needed(&self) -> SemanticNeed {
        self.semantic_needed
    }
}
/// Identity is mandatory: readiness layout comes from the same digest/producer
/// preparation used by the warm store, never from an empty or process-local default.
pub(crate) fn prepare(
    compiled: Compiled,
    rules: &[RuleSpec],
    rule_digests: &[ryo_values::domain::stored_facts::types::Digest],
    hosted: &[HostRelation],
    derived: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
    identity: ObservationIdentity,
    routes: crate::domain::executable_program::types::RoutePolicy,
) -> Result<Arc<PreparedProgram>, OperatingError> {
    let derived: crate::application::file_dag::derived::owner::types::DerivedAccess = derived.into();
    if rule_digests.len() != rules.len() {
        return Err(OperatingError::WrongOperation);
    }
    let Compiled {
        plans,
        bindings,
        schedule,
        observations,
    } = compiled;
    let mut walk = crate::domain::shared_walk::types::prepared_walk::value::PreparedWalk::compile(
        &plans, &bindings,
    );
    let source_plans: Vec<_> = plans.iter().map(Ok).collect();
    let partitions = SourcePartitions::new(rules, &source_plans, &bindings, hosted, &observations)?;
    let operators = crate::application::file_dag::service::prepare(&plans, &bindings, hosted, derived.clone())?;
    let request = match identity {
        ObservationIdentity::Independent => {
            crate::domain::file_program::types::ObservationRequest::Independent
        }
        ObservationIdentity::Named(layout) => {
            crate::domain::file_program::service::observations::request(
                &schedule,
                &observations,
                layout,
            )
            .map_err(|error| OperatingError::ObservationIncomplete {
                reason: error.to_string(),
            })?
        }
    };
    let graph = crate::domain::file_program::service::compile_prepared_observations(
        &plans, &operators, request,
    );
    let named = graph
        .observations
        .as_ref()
        .map(|named| ObservationMetadata {
            layout: named.layout,
            slots: named.slots.clone().into_boxed_slice(),
            frontiers: named.frontiers.clone().into_boxed_slice(),
            bindings: observations.clone(),
            outputs: named.outputs.iter().map(|outputs| outputs.iter().map(|output| OutputMetadata {
                root: match output.root { crate::domain::file_program::types::GraphObservationRoot::PerFile(_) => OutputRoot::PerFile,
                    crate::domain::file_program::types::GraphObservationRoot::CrossFile => OutputRoot::CrossFile,
                    crate::domain::file_program::types::GraphObservationRoot::Unavailable => OutputRoot::Unavailable }, consumers: output.consumers.iter().copied().map(RuleIndex).collect(),
            }).collect()).collect(),
            retire: named.retire.clone(),
            consumers: named
                .outputs
                .iter()
                .map(|outputs| {
                    outputs
                        .iter()
                        .map(|output| output.consumers.iter().copied().map(RuleIndex).collect())
                        .collect()
                })
                .collect(),
        });
    let observation_frontiers: Vec<Vec<FrontierPurpose>> = graph
        .observations
        .as_ref()
        .map(|named| named.observation_frontiers.clone())
        .unwrap_or_else(|| {
            plans
                .iter()
                .map(|plan| vec![false; plan.frontier.len()])
                .collect()
        })
        .into_iter()
        .map(|rule| {
            rule.into_iter()
                .map(|observed| {
                    if observed {
                        FrontierPurpose::Observation
                    } else {
                        FrontierPurpose::Diagnostic
                    }
                })
                .collect()
        })
        .collect();
    let program = Arc::new(crate::domain::executable_program::service::compile(
        &plans,
        graph,
        &partitions,
        &operators,
        routes,
    )?);
    walk.retain_program_consumers(&program)?;
    census::service::trace(rules, &walk, &program);
    let native_orders = orders::service::prepare(&program, &derived)?;
    let derived = crate::application::file_dag::derived::prepared::RequestDerived::compile(
        &program, &operators,
    )?;
    let metadata =
        metadata::service::rules(&program, named.as_ref(), &plans, &observation_frontiers)?;
    let matchers = Arc::new(
        crate::domain::matching::types::prepared::value::PreparedMatchers::from_operations(
            program.regions().iter().flat_map(|region| {
                region
                    .instructions()
                    .iter()
                    .map(|instruction| instruction.operation())
            }),
        ),
    );
    let file_demands = demands::service::file_demands(&program, &metadata)?;
    let file_frontier_outputs = demands::service::frontier_outputs(&metadata, named.as_ref());
    let semantic_needed = if metadata
        .iter()
        .any(|rule| rule.semantic_needed == SemanticNeed::Required)
    {
        SemanticNeed::Required
    } else {
        SemanticNeed::NotRequired
    };
    Ok(Arc::new(PreparedProgram {
        program,
        native_orders,
        rule_ids: rules.iter().map(|rule| rule.id.clone()).collect(),
        applicability: rules.iter().map(|rule| rule.applicability).collect(),
        rule_digests: rule_digests.to_vec().into_boxed_slice(),
        bindings: bindings.into_iter().map(Vec::into_boxed_slice).collect(),
        rules: metadata.into_boxed_slice(),
        semantic_needed,
        file_demands,
        file_frontier_outputs,
        observations: named,
        schedule,
        semantic_metadata: ryo_values::domain::run_metadata::service::from_plans(&plans),
        hosted: hosted.to_vec().into_boxed_slice(),
        matchers,
        derived,
        walk,
    }))
}
