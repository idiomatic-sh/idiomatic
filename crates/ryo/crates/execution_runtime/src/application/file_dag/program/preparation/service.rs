//! Immutable request preparation, physical metadata and compilation ownership.
use super::types::*;
use crate::application::file_dag::program::prepared_execution::types::{
    NamedLayout, ObservationContract,
};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::interpreted_program::types::interpreted_program::value::InterpretedProgram;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::prepared_request::types::Compiled;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
use ryo_execution_domain::domain::rule_intent::types::FrontierPurpose;
use ryo_execution_domain::domain::rule_intent::types::SemanticNeed;
use ryo_execution_domain::domain::source_partitions::types::ProducerPartitions as _;
use ryo_execution_domain::domain::source_partitions::types::source_partitions::value::SourcePartitions;
use ryo_execution_domain::domain::trust::types::RelationRead;

use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::query_bindings::types::ParameterBinding;
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
    /// The interpreter's compile, for the project phase alone.
    ///
    /// # Errors
    ///
    /// The fault the interpreter's compile of the request answered.
    pub(crate) fn interpreted(&self) -> Result<&Interpreted, OperatingError> {
        self.interpreted.as_ref().map_err(Clone::clone)
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
    /// The relations each file's sources are asked for: those the request's plans read.
    pub(crate) fn demanded(&self) -> &[ryo_relations::domain::relations::types::Relation] {
        &self.demanded
    }
    pub(crate) fn hosted(&self) -> &[HostRelation] {
        &self.hosted
    }
    pub(crate) fn matchers(
        &self,
    ) -> Result<
        &Arc<ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers>,
        OperatingError,
    > {
        Ok(&self.interpreted()?.matchers)
    }
    pub(crate) fn rule_metadata(&self, rule: RuleIndex) -> Result<&RuleMetadata, OperatingError> {
        tracing::trace!(
            target: ryo_execution_store::application::file_dag::derived::constants::TARGET,
            event = ryo_execution_store::application::file_dag::derived::constants::RULE_METADATA,
            rule = rule.0
        );
        self.interpreted()?
            .rules
            .get(rule.0)
            .ok_or(OperatingError::NotRun)
    }
    /// What `rule`'s plan states.
    ///
    /// # Errors
    ///
    /// `NotRun` for a rule the request does not hold.
    pub(crate) fn rule_plan(&self, rule: RuleIndex) -> Result<&RulePlan, OperatingError> {
        self.rules.get(rule.0).ok_or(OperatingError::NotRun)
    }
    pub(crate) fn observation_metadata(&self) -> Option<&ObservationMetadata> {
        match &self.interpreted {
            Ok(interpreted) => interpreted.observations.as_ref(),
            Err(_) => None,
        }
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
/// Identity is mandatory: readiness layout comes from the same digest/producer
/// preparation used by the warm store, never from an empty or process-local default.
/// The executable program of a request: the one place outside the domain that compiles one, so a change to the domain's
/// compilation edits this function alone.
pub(crate) fn compile_program(
    plans: &[ryo_values::domain::plans::types::Plan],
    graph: ryo_execution_domain::domain::file_program::types::RequestGraph,
    partitions: &dyn ryo_execution_domain::domain::source_partitions::types::ProducerPartitions,
    operators: &ryo_execution_domain::domain::file_program::types::RequestOperators,
) -> Result<InterpretedProgram, OperatingError> {
    ryo_execution_domain::domain::interpreted_program::service::compile(
        plans,
        graph,
        partitions,
        operators,
        ryo_operators::domain::patterns::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
    )
}
pub(crate) fn prepare(
    compiled: Compiled,
    rules: &[RuleSpec],
    rule_digests: &[ryo_values::domain::stored_facts::types::Digest],
    hosted: &[HostRelation],
    derived: impl Into<crate::application::file_dag::derived::program::service::ProgramDerived>,
    identity: ObservationIdentity,
    pass: &crate::application::file_dag::file_pass::types::PassProgram,
) -> Result<Arc<PreparedProgram>, OperatingError> {
    use ryo_execution_domain::domain::observation_schedule::service::frontiers::service::{
        frontier_purposes, observed_frontiers, output_consumers,
    };
    use ryo_execution_domain::domain::observation_schedule::types::ObservationRequest;
    let derived: crate::application::file_dag::derived::program::service::ProgramDerived =
        derived.into();
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
        ObservationRequest::Bound { layout, .. } => Some(ObservedLayout {
            layout,
            frontiers: observed_frontiers(&owned).into_boxed_slice(),
            bindings: observations.clone(),
            consumers: output_consumers(&owned, &observations),
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
    let rule_plans = metadata::service::plans(&owned, &pass.reads, &purposes)?;
    let file_frontier_outputs = demands::service::frontier_outputs(&rule_plans, observed.as_ref());
    let semantic_needed = if rule_plans
        .iter()
        .any(|rule| rule.semantic_needed == SemanticNeed::Required)
    {
        SemanticNeed::Required
    } else {
        SemanticNeed::NotRequired
    };
    // The interpreter's compile serves the project phase alone; its fault refuses that phase, never the request.
    let interpreted = interpreted(Interpreting {
        plans: owned.clone(),
        bindings: &bindings,
        rules,
        hosted,
        derived,
        observations: &observations,
        request,
        rule_plans: &rule_plans,
        executable: &pass.program,
        partitions,
    });
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
        demanded: crate::application::file_dag::file_pass::service::demanded(&owned),
        interpreted,
    }))
}

/// The interpreter's compile of a request, for the project phase: its walk, operators, observation graph and program, and each
/// rule's regions.
fn interpreted(input: Interpreting<'_>) -> Result<Interpreted, OperatingError> {
    let Interpreting {
        plans,
        bindings,
        rules,
        hosted,
        derived,
        observations,
        request,
        rule_plans,
        executable,
        partitions,
    } = input;
    let operators =
        crate::application::file_dag::service::prepare(&plans, bindings, hosted, derived)?;
    let graph = ryo_execution_domain::domain::file_program::service::compile_prepared_observations(
        &plans, &operators, request,
    );
    let named = graph
        .observations
        .as_ref()
        .map(|named| ObservationMetadata {
            layout: named.layout,
            frontiers: named.frontiers.clone().into_boxed_slice(),
            bindings: observations.to_vec(),
            admissions: observations
                .iter()
                .enumerate()
                .map(|(rule, bindings)| {
                    bindings
                        .iter()
                        .map(|binding| partitions.observation(RuleIndex(rule), binding.input).map(|_| ()))
                        .collect()
                })
                .collect(),
            outputs: named.outputs.iter().map(|outputs| outputs.iter().map(|output| OutputMetadata {
                root: match output.root { ryo_execution_domain::domain::file_program::types::GraphObservationRoot::PerFile(_) => OutputRoot::PerFile,
                    ryo_execution_domain::domain::file_program::types::GraphObservationRoot::CrossFile => OutputRoot::CrossFile,
                    ryo_execution_domain::domain::file_program::types::GraphObservationRoot::Unavailable => OutputRoot::Unavailable }, consumers: output.consumers.clone(),
            }).collect()).collect(),
            retire: named.retire.clone(),
        });
    let program = Arc::new(compile_program(&plans, graph, &partitions, &operators)?);
    let metadata =
        metadata::service::rules(&program, named.as_ref(), &plans, rule_plans, executable)?;
    // The program's own pattern slots: its presence fold compiled the source searches' patterns into them.
    let matchers = Arc::clone(program.matchers());
    Ok(Interpreted {
        rules: metadata.into_boxed_slice(),
        observations: named,
        matchers,
    })
}
