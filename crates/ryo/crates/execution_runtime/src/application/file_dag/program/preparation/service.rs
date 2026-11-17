//! Immutable request preparation, physical metadata and compilation ownership.
use super::types::*;
use crate::application::file_dag::program::prepared_execution::types::{
    NamedLayout, ObservationContract,
};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::prepared_request::types::Compiled;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
use ryo_execution_domain::domain::rule_intent::types::FrontierPurpose;
use ryo_execution_domain::domain::rule_intent::types::SemanticNeed;
use ryo_execution_domain::domain::source_partitions::types::ProducerPartitions as _;
use ryo_execution_domain::domain::source_partitions::types::source_partitions::value::SourcePartitions;
use ryo_execution_domain::domain::trust::types::relation_read::value::RelationRead;

use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::records::types::ObservationIndex;
use ryo_values::domain::rule_requests::types::RuleSpec;
use std::sync::Arc;

mod demands;
mod metadata;
mod replay;
mod schedule;

impl PreparedProgram {
    pub(crate) fn file_frontier_outputs(
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
    pub(crate) fn rule_id(
        &self,
        rule: RuleIndex,
    ) -> Result<&ryo_values::domain::runs::types::RuleId, OperatingError> {
        self.rule_ids.get(rule.0).ok_or(OperatingError::NotRun)
    }
    pub(crate) fn applicability(
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
    /// The relations `rule`'s steps read, each once, in the order first read.
    ///
    /// # Errors
    ///
    /// `NotRun` for a rule the request does not hold.
    pub(crate) fn reads(
        &self,
        rule: RuleIndex,
    ) -> Result<&ryo_values::domain::plans::types::RuleReads, OperatingError> {
        self.reads.get(rule.0).ok_or(OperatingError::NotRun)
    }
    pub(crate) fn hosted(&self) -> &[HostRelation] {
        &self.hosted
    }
    /// What `rule`'s plan states.
    ///
    /// # Errors
    ///
    /// `NotRun` for a rule the request does not hold.
    pub(crate) fn rule_plan(&self, rule: RuleIndex) -> Result<&RulePlan, OperatingError> {
        tracing::trace!(
            target: super::constants::TARGET,
            event = super::constants::RULE_METADATA,
            rule = rule.0
        );
        self.rules.get(rule.0).ok_or(OperatingError::NotRun)
    }
    pub(crate) fn observation_metadata(&self) -> Option<&ObservationMetadata> {
        self.observed.as_ref()
    }
    pub(crate) fn emit(&self, rule: RuleIndex) -> Result<&Emit, OperatingError> {
        Ok(&self.rule_plan(rule)?.emit)
    }
    pub(crate) fn observation_layout(&self) -> Option<NamedLayout<'_>> {
        self.observed.as_ref().map(|observations| NamedLayout {
            layout: observations.layout,
            frontiers: observations.frontiers.as_ref(),
        })
    }
    pub(crate) fn frontier_purpose(
        &self,
        rule: RuleIndex,
        input: InputNumber,
    ) -> Result<FrontierPurpose, OperatingError> {
        self.rule_plan(rule)?
            .observation_frontiers
            .get(input.0)
            .copied()
            .ok_or(OperatingError::NotRun)
    }
    pub(crate) fn frontiers(&self, rule: RuleIndex) -> Result<&[Input], OperatingError> {
        Ok(&self.rule_plan(rule)?.frontiers)
    }
    pub(crate) fn frontier_dependencies(
        &self,
        rule: RuleIndex,
    ) -> Result<&[Vec<ryo_values::domain::plan_parts::types::ObservationBindingId>], OperatingError>
    {
        self.rule_plan(rule)?
            .frontier_dependencies
            .as_deref()
            .map_err(Clone::clone)
    }
    pub(crate) fn has_observation_contract(
        &self,
        rule: RuleIndex,
    ) -> Result<ObservationContract, OperatingError> {
        let declared = !self.rule_plan(rule)?.roots.is_empty()
            || self.observed.as_ref().is_some_and(|named| {
                named
                    .bindings
                    .get(rule.0)
                    .is_some_and(|bound| !bound.is_empty())
            });
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
            .rule_plan(rule)?
            .roots
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
pub(crate) fn prepare(
    compiled: Compiled,
    rules: &[RuleSpec],
    rule_digests: &[ryo_values::domain::stored_facts::types::Digest],
    hosted: &[HostRelation],
    identity: ObservationIdentity,
    pass: &crate::application::file_dag::file_pass::types::PassProgram,
    sets: &mut ryo_execution_domain::domain::trust::types::read_sets::value::ReadSets,
) -> Result<Arc<PreparedProgram>, OperatingError> {
    use ryo_execution_domain::domain::observation_schedule::service::frontiers::service::{
        frontier_purposes, observed_frontiers, output_consumers, retirements,
    };
    use ryo_execution_domain::domain::observation_schedule::types::ObservationRequest;
    if rule_digests.len() != rules.len() {
        return Err(OperatingError::WrongOperation);
    }
    let Compiled {
        plans,
        bindings,
        schedule,
        observations,
    } = compiled;
    // The request's declarations, one per rule: a request whose plans, bindings or observations do not match its rules is
    // refused first, whatever the interpreter's compile does.
    let source_plans: Vec<Result<&ryo_values::domain::plans::types::Plan, OperatingError>> =
        plans.iter().map(|plan| Ok(plan.as_ref())).collect();
    let partitions = SourcePartitions::new(rules, &source_plans, &bindings, hosted, &observations)?;
    let request = match identity {
        ObservationIdentity::Independent => ObservationRequest::Independent,
        ObservationIdentity::Named(layout) => {
            ryo_execution_domain::domain::observation_schedule::service::request(
                &schedule,
                &observations,
                layout,
            )
            .map_err(|error| OperatingError::ObservationIncomplete {
                reason: error.to_string(),
            })?
        }
    };
    // What the plans and the observation request state, read once for the file pass and the request's port.
    let shared: Vec<&ryo_values::domain::plans::types::Plan> =
        plans.iter().map(AsRef::as_ref).collect();
    let owned: Vec<ryo_values::domain::plans::types::Plan> =
        shared.iter().map(|plan| (*plan).clone()).collect();
    let observed = match request {
        ObservationRequest::Bound { layout, .. } => Some(ObservationMetadata {
            layout,
            frontiers: observed_frontiers(&owned).into_boxed_slice(),
            admissions: observations
                .iter()
                .enumerate()
                .map(|(rule, bindings)| {
                    bindings
                        .iter()
                        .map(|binding| {
                            partitions
                                .observation(RuleIndex(rule), binding.input)
                                .map(|_| ())
                        })
                        .collect()
                })
                .collect(),
            outputs: outputs(
                &owned,
                &output_consumers(&owned, &observations),
                &pass.program,
            )?,
            retire: retirements(&observations),
            bindings: observations.clone(),
        }),
        ObservationRequest::Independent => None,
    };
    let purposes = if observed.is_some() {
        frontier_purposes(&owned)
    } else {
        owned
            .iter()
            .map(|plan| vec![FrontierPurpose::Diagnostic; plan.frontier.len()])
            .collect()
    };
    let rule_plans = metadata::service::plans(&owned, pass, sets, &purposes)?;
    let file_frontier_outputs = demands::service::frontier_outputs(&rule_plans, observed.as_ref());
    let semantic_needed = if rule_plans
        .iter()
        .any(|rule| rule.semantic_needed == SemanticNeed::Required)
    {
        SemanticNeed::Required
    } else {
        SemanticNeed::NotRequired
    };
    Ok(Arc::new(PreparedProgram {
        rule_ids: rules.iter().map(|rule| rule.id.clone()).collect(),
        applicability: rules.iter().map(|rule| rule.applicability).collect(),
        rule_digests: rule_digests.to_vec().into_boxed_slice(),
        rules: rule_plans.into_boxed_slice(),
        semantic_needed,
        file_frontier_outputs,
        observed,
        schedule,
        semantic_metadata: ryo_values::domain::run_metadata::service::from_plans(&owned),
        hosted: hosted.to_vec().into_boxed_slice(),
        reads: owned
            .iter()
            .map(ryo_values::domain::plans::service::reads_of)
            .collect(),
    }))
}

/// Each named output of each plan, where it is made and which rules consume it: after the files, in each file by the step the
/// executable program made of its source, or unavailable when the program made no node of it.
///
/// # Errors
///
/// `NotRun` for a rule the executable program does not hold.
fn outputs(
    plans: &[ryo_values::domain::plans::types::Plan],
    consumers: &[Vec<Vec<RuleIndex>>],
    executable: &ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram,
) -> Result<Vec<Vec<OutputMetadata>>, OperatingError> {
    plans
        .iter()
        .enumerate()
        .map(|(rule, plan)| {
            let nodes = ryo_execution_domain::domain::executable_program::service::rule(
                executable,
                RuleIndex(rule),
            )?;
            Ok(plan
                .observations
                .iter()
                .enumerate()
                .map(|(output, observation)| OutputMetadata {
                    root: if observation.scope
                        == ryo_values::domain::plan_parts::types::StepScope::CrossFile
                    {
                        OutputRoot::CrossFile
                    } else if let Input::Step(step) = observation.source
                        && nodes
                            .steps
                            .get(usize::try_from(step.0).unwrap_or(usize::MAX))
                            .is_some_and(Result::is_ok)
                    {
                        OutputRoot::PerFile
                    } else {
                        OutputRoot::Unavailable
                    },
                    consumers: consumers
                        .get(rule)
                        .and_then(|outputs| outputs.get(output))
                        .cloned()
                        .unwrap_or_default(),
                })
                .collect())
        })
        .collect()
}
