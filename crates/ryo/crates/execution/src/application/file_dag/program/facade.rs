//! Application request preparation and compact root metadata; original plans are transient.
pub(crate) use super::admission::ApplicabilityReason;
pub(crate) use super::restored::ValidatedFrontier;
pub(in crate::application) use super::restored::validate_restored_frontier;
use crate::domain::prepared_request::types::{Compiled, ObservationIdentity};
use crate::domain::source_partitions::types::source_partitions::value::SourcePartitions;
use crate::domain::{
    coverage::types::RuleIndex,
    executable_program::types::{ExecutableProgram, InputSource, RegionId},
    file_program::types::{
        ObservationSlot, ObservedFrontierRoot, ResolvedObservationBinding, RuleSchedule,
    },
    operating::errors::OperatingError,
    operator::{contract::types::RequestOperators, types::RowsFed},
    trust::types::RelationRead,
};
use crate::outbound::derived::interface::DerivedPort;
use ryo_relations::domain::relations::types::{CellKind, Words};
use ryo_values::domain::{
    hosted::types::HostRelation,
    observations::types::{ObservationIndex, ObservationReadinessLayout},
    plan_parts::types::{Input, StepId},
    plans::types::{Emit, StepScope},
    rows::types::Table,
};
use std::sync::Arc;

pub(crate) use super::super::scope::ScopePartials;
pub(crate) use super::admission::RequiredPartitions;
pub(crate) use super::facade_runtime::{
    FileRun, ProgramServices, ProjectRun, PublicationScope, open_file, open_project,
    publication_owner, required_partitions,
};
pub struct CaptureContext<'a> {
    pub file: ryo_relations::domain::relations::types::FileId,
    /// Current rule identity validated by the store before warm replay.
    pub rule_digest: ryo_values::domain::stored_facts::types::Digest,
    pub worker: ryo_relations::domain::relations::types::WorkerNumber,
    pub retained: crate::domain::finish::types::Retained<'a>,
    pub words: &'a mut Words,
}
pub(crate) struct ProjectFrontiers<'a> {
    pub publication: &'a PublicationScope,
    pub ordinary: crate::application::file_dag::frontiers::types::PreparedFrontiers,
    pub bound_inputs: &'a [crate::application::file_dag::frontiers::types::BoundInput],
}
pub struct FrontierCapture {
    pub input: ryo_values::domain::plan_parts::types::InputNumber,
    pub outcome: Result<CapturedFrontier, OperatingError>,
}
pub struct CapturedFrontier {
    pub input: ryo_values::domain::plan_parts::types::InputNumber,
    pub bucket: ryo_values::domain::stored_facts::types::Bucket,
    /// Source-bound root and physical coverage retained with the packed words.
    pub proof: ryo_values::domain::records::types::FrontierProof,
    pub words: ryo_values::domain::stored_facts::types::PackedWords,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmitDemand {
    Enabled,
    Disabled,
}
/// Diagnostic output delivery, fixed before admission. Every diagnostic,
/// observation and frontier uses the same semantic evaluation contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticDelivery {
    Disabled,
    Collected,
    Forwarded,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationDemand {
    Required,
    NotRequired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleDemand {
    pub emit: EmitDemand,
    pub observations: ObservationDemand,
}
/// Frontier roots admitted by this file generation. Observation-only producers
/// cannot acquire diagnostic-only frontiers through a later capture call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureIntent {
    Disabled,
    Observations,
    DiagnosticAndObservations,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicabilityRefusal {
    SyntaxUndetermined,
    MetadataUnavailable,
}
/// Complete per-rule intent, frozen before any file source can be walked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileRuleIntent {
    Undemanded,
    Skipped(ApplicabilityReason),
    Refused(ApplicabilityRefusal),
    Run {
        diagnostic: DiagnosticDelivery,
        observations: ObservationDemand,
        capture: CaptureIntent,
    },
}
pub enum RootOutcome {
    Complete(Table),
    /// Usable diagnostic rows and the exact physical partitions still uncertain.
    Partial {
        table: Table,
        affected: Vec<Option<ryo_relations::domain::relations::types::FileId>>,
    },
    Skipped(super::admission::ApplicabilityProof),
    Unknown(OperatingError),
    Failed(OperatingError),
    NoQuery,
    /// This compiled root belongs to another execution phase and has not run.
    Deferred {
        phase: StepScope,
    },
}
/// Opaque application handover of the actual admitted population. Packed bytes
/// may accompany it, but never substitute for its owner/generation evidence.
#[derive(Clone)]
pub struct ObservedPublication {
    admitted: super::admission::AdmittedInput,
}
impl ObservedPublication {
    pub(super) fn from_admitted(admitted: super::admission::AdmittedInput) -> Self {
        Self { admitted }
    }
    pub(super) fn admitted(&self) -> &super::admission::AdmittedInput {
        &self.admitted
    }
}
pub struct LogicalCounts {
    pub fed: Box<[RowsFed]>,
}
pub struct RuleOutcome {
    pub emit: RootOutcome,
    pub observations: Vec<(ObservationIndex, RootOutcome)>,
    pub counts: LogicalCounts,
}
/// A project named root can be Complete only with the admitted owner proof.
pub enum ProjectObservation {
    Complete {
        table: Table,
        publication: ObservedPublication,
    },
    Other(RootOutcome),
}
pub struct ProjectRuleOutcome {
    pub emit: RootOutcome,
    pub observations: Vec<(ObservationIndex, ProjectObservation)>,
    pub counts: LogicalCounts,
}
pub(super) struct RootMetadata {
    pub source: Result<InputSource, OperatingError>,
    pub phase: StepScope,
    pub reads: Box<[RelationRead]>,
}
pub(super) struct RuleMetadata {
    pub emit: Emit,
    pub emit_root: Result<RegionId, OperatingError>,
    pub frontiers: Box<[Input]>,
    pub observations: Box<[RootMetadata]>,
    pub observation_frontiers: Box<[bool]>,
    pub semantic_needed: bool,
    pub frontier_dependencies: Result<
        Vec<Vec<ryo_values::domain::observations::types::ObservationBindingId>>,
        OperatingError,
    >,
}
#[derive(Clone, Copy)]
pub(in crate::application) enum OutputRoot {
    PerFile,
    CrossFile,
    Unavailable,
}
pub(in crate::application) struct OutputMetadata {
    pub root: OutputRoot,
    pub consumers: Vec<usize>,
}
pub(crate) struct ObservationMetadata {
    pub(in crate::application) layout: ObservationReadinessLayout,
    pub(in crate::application) slots: Box<[ObservationSlot]>,
    pub(in crate::application) frontiers: Box<[ObservedFrontierRoot]>,
    pub(in crate::application) bindings: Vec<Vec<ResolvedObservationBinding>>,
    pub(in crate::application) consumers: Vec<Vec<Vec<usize>>>,
    pub(in crate::application) outputs: Vec<Vec<OutputMetadata>>,
    pub(in crate::application) retire:
        Vec<Vec<crate::domain::file_program::types::ObservationRetirement>>,
}
/// Immutable executable state has no RequestGraph, Plan, layout or old executor.
pub(crate) struct PreparedProgram {
    program: Arc<ExecutableProgram>,
    rule_ids: Box<[ryo_values::domain::runs::types::RuleId]>,
    applicability: Box<[ryo_values::domain::runs::types::RuleApplicability]>,
    rule_digests: Box<[ryo_values::domain::stored_facts::types::Digest]>,
    bindings: Box<[Box<[ParameterBinding]>]>,
    rules: Box<[RuleMetadata]>,
    observations: Option<ObservationMetadata>,
    schedule: RuleSchedule,
    semantic_metadata: Vec<ryo_values::domain::run_metadata::types::RuleMetadata>,
    hosted: Box<[HostRelation]>,
    matchers: Arc<crate::domain::matching::types::prepared::value::PreparedMatchers>,
    derived: super::super::derived::prepared::PreparedDerived,
    native_orders: Box<[Option<Arc<ryo_derived::domain::contract::types::PreparedDerived>>]>,
    walk: crate::domain::shared_walk::types::prepared_walk::value::PreparedWalk,
}
impl PreparedProgram {
    pub(super) fn native_order(
        &self,
        region: crate::domain::executable_program::types::RegionId,
    ) -> Result<Option<Arc<ryo_derived::domain::contract::types::PreparedDerived>>, OperatingError>
    {
        self.native_orders
            .get(region.0)
            .cloned()
            .ok_or(OperatingError::NotRun)
    }

    pub(super) fn walk(
        &self,
    ) -> &crate::domain::shared_walk::types::prepared_walk::value::PreparedWalk {
        &self.walk
    }
    pub(super) fn all_bindings(&self) -> &[Box<[ParameterBinding]>] {
        &self.bindings
    }
    pub(super) fn walk_class(
        &self,
        instruction: &crate::domain::executable_program::types::Instruction,
    ) -> Option<crate::domain::positions::types::ClassPlace> {
        self.walk.class_for(instruction)
    }
    pub(super) fn rule_id(
        &self,
        rule: RuleIndex,
    ) -> Result<&ryo_values::domain::runs::types::RuleId, OperatingError> {
        self.rule_ids.get(rule.0).ok_or(OperatingError::NotRun)
    }
    pub(super) fn applicability(
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
    pub(super) fn derived(&self) -> &super::super::derived::prepared::PreparedDerived {
        &self.derived
    }
    pub(super) fn hosted(&self) -> &[HostRelation] {
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
    pub(super) fn rule_metadata(&self, rule: RuleIndex) -> Result<&RuleMetadata, OperatingError> {
        self.rules.get(rule.0).ok_or(OperatingError::NotRun)
    }
    pub(crate) fn observation_metadata(&self) -> Option<&ObservationMetadata> {
        self.observations.as_ref()
    }
    pub(crate) fn emit(&self, rule: RuleIndex) -> Result<&Emit, OperatingError> {
        Ok(&self.rule_metadata(rule)?.emit)
    }
    pub(crate) fn observation_layout(
        &self,
    ) -> Option<(ObservationReadinessLayout, &[ObservedFrontierRoot])> {
        self.observations
            .as_ref()
            .map(|observations| (observations.layout, observations.frontiers.as_ref()))
    }
    pub(crate) fn captures_observation(
        &self,
        rule: RuleIndex,
        input: usize,
    ) -> Result<bool, OperatingError> {
        self.rule_metadata(rule)?
            .observation_frontiers
            .get(input)
            .copied()
            .ok_or(OperatingError::NotRun)
    }
    pub(crate) fn frontiers(&self, rule: RuleIndex) -> Result<&[Input], OperatingError> {
        Ok(&self.rule_metadata(rule)?.frontiers)
    }
    pub(crate) fn frontier_dependencies(
        &self,
        rule: RuleIndex,
    ) -> Result<&[Vec<ryo_values::domain::observations::types::ObservationBindingId>], OperatingError>
    {
        self.rule_metadata(rule)?
            .frontier_dependencies
            .as_deref()
            .map_err(Clone::clone)
    }
    pub(crate) fn has_observation_contract(&self, rule: RuleIndex) -> Result<bool, OperatingError> {
        Ok(!self.rule_metadata(rule)?.observations.is_empty()
            || self
                .observations
                .as_ref()
                .is_some_and(|named| !named.bindings[rule.0].is_empty()))
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
    pub(crate) fn metadata(&self) -> &[ryo_values::domain::run_metadata::types::RuleMetadata] {
        &self.semantic_metadata
    }

    pub(crate) fn replay_closure(
        &self,
        again: ryo_values::domain::stores::types::Reuse,
        first: &[ryo_values::domain::stored_facts::types::Bucket],
    ) -> Result<
        (
            ryo_values::domain::stores::types::Reuse,
            Vec<ryo_values::domain::stored_facts::types::Bucket>,
        ),
        OperatingError,
    > {
        use {
            ryo_values::domain::stores::types::MadeAgain, ryo_values::domain::stores::types::Reuse,
        };
        let Some(named) = &self.observations else {
            return Ok((again, vec![]));
        };
        let Reuse::Cross(MadeAgain::These(selected)) = again else {
            return Ok((again, vec![]));
        };
        let mut needed = vec![false; self.rules.len()];
        let mut queue = std::collections::VecDeque::new();
        for selected in selected {
            let rule = RuleIndex(selected.0 as usize);
            *needed.get_mut(rule.0).ok_or(OperatingError::NotRun)? = true;
            queue.push_back(rule);
        }
        while let Some(rule) = queue.pop_front() {
            for output in named.consumers.get(rule.0).ok_or(OperatingError::NotRun)? {
                for &consumer in output {
                    let mark = needed.get_mut(consumer).ok_or(OperatingError::NotRun)?;
                    if !*mark {
                        *mark = true;
                        queue.push_back(RuleIndex(consumer));
                    }
                }
            }
            for binding in named.bindings.get(rule.0).ok_or(OperatingError::NotRun)? {
                for producer in &binding.producers {
                    if let crate::domain::file_program::types::ResolvedObservationProducer::Output { rule, output } = producer {
                        let root = self.rule_metadata(*rule)?.observations.get(output.0 as usize).ok_or(OperatingError::NotRun)?;
                        if root.phase == StepScope::CrossFile {
                            let mark = needed.get_mut(rule.0).ok_or(OperatingError::NotRun)?;
                            if !*mark { *mark = true; queue.push_back(*rule); }
                        }
                    }
                }
            }
        }
        let mut buckets = std::collections::BTreeSet::new();
        for root in named.frontiers.iter() {
            let consumers = named
                .consumers
                .get(root.producer.0)
                .and_then(|outputs| outputs.get(root.output.0 as usize))
                .ok_or(OperatingError::NotRun)?;
            if consumers.iter().any(|consumer| needed[*consumer]) {
                buckets.insert(ryo_values::domain::stored_facts::service::frontier_bucket(
                    *first.get(root.producer.0).ok_or(OperatingError::NotRun)?,
                    root.frontier,
                ));
            }
        }
        let selected = needed
            .into_iter()
            .enumerate()
            .filter(|(_, needed)| *needed)
            .map(|(rule, _)| {
                u32::try_from(rule)
                    .map(ryo_values::domain::records::types::PlacedRule)
                    .map_err(|_| OperatingError::NotRun)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((
            Reuse::Cross(MadeAgain::These(selected)),
            buckets.into_iter().collect(),
        ))
    }
    pub(crate) fn observation_batches(&self) -> Option<&[Vec<RuleIndex>]> {
        match &self.schedule {
            RuleSchedule::Independent => None,
            RuleSchedule::Dependencies { batches, .. } => Some(batches),
        }
    }
    pub(crate) fn rule_count(&self) -> usize {
        self.rules.len()
    }
    pub(crate) fn semantic_needed(&self) -> bool {
        self.rules.iter().any(|rule| rule.semantic_needed)
    }
    pub(crate) fn rule_order(&self) -> impl Iterator<Item = RuleIndex> + '_ {
        let order = match &self.schedule {
            RuleSchedule::Independent => None,
            RuleSchedule::Dependencies { order, .. } => Some(order.as_slice()),
        };
        order
            .into_iter()
            .flatten()
            .copied()
            .chain((0..if order.is_none() { self.rules.len() } else { 0 }).map(RuleIndex))
    }
    pub(crate) fn demanded_rules(&self, governs: impl Fn(usize) -> bool) -> Vec<bool> {
        let mut demanded: Vec<_> = (0..self.rules.len()).map(governs).collect();
        if let Some(observations) = &self.observations {
            for consumer in self.rule_order().collect::<Vec<_>>().into_iter().rev() {
                if !demanded[consumer.0] {
                    continue;
                }
                for binding in &observations.bindings[consumer.0] {
                    for producer in &binding.producers {
                        if let crate::domain::file_program::types::ResolvedObservationProducer::Output { rule, .. } = producer {
                            demanded[rule.0] = true;
                        }
                    }
                }
            }
        }
        demanded
    }
}
/// Identity is mandatory: readiness layout comes from the same digest/producer
/// preparation used by the warm store, never from an empty or process-local default.
pub(crate) fn prepare(
    compiled: Compiled,
    rules: &[RuleSpec],
    rule_digests: &[ryo_values::domain::stored_facts::types::Digest],
    hosted: &[HostRelation],
    derived: Arc<dyn DerivedPort>,
    identity: ObservationIdentity,
) -> Result<Arc<PreparedProgram>, OperatingError> {
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
    let operators = RequestOperators::prepare(&plans, &bindings, hosted, Arc::clone(&derived))?;
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
                    crate::domain::file_program::types::GraphObservationRoot::Unavailable => OutputRoot::Unavailable }, consumers: output.consumers.clone(),
            }).collect()).collect(),
            retire: named.retire.clone(),
            consumers: named
                .outputs
                .iter()
                .map(|outputs| {
                    outputs
                        .iter()
                        .map(|output| output.consumers.clone())
                        .collect()
                })
                .collect(),
        });
    let observation_frontiers: Vec<_> = graph
        .observations
        .as_ref()
        .map(|named| named.observation_frontiers.clone())
        .unwrap_or_else(|| {
            plans
                .iter()
                .map(|plan| vec![false; plan.frontier.len()])
                .collect()
        });
    let program = Arc::new(crate::domain::executable_program::service::compile(
        &plans,
        graph,
        &partitions,
        &operators,
    )?);
    walk.retain_program_consumers(&program)?;
    let native_orders = program
        .regions()
        .iter()
        .map(|region| {
            if region.native_positions.is_none() {
                return None;
            }
            use ryo_values::domain::plan_parts::types::RepeatedRows;
            use ryo_values::domain::plans::types::Operation;
            let stride = match &region.instructions[0].operation {
                Operation::Project {
                    columns,
                    distinct: RepeatedRows::Dropped,
                    ..
                } => Some(columns.len().max(1)),
                Operation::Union { .. } => region
                    .native_positions
                    .as_ref()
                    .map(|shape| shape.columns.len().max(1)),
                Operation::Antijoin { right_on, .. } | Operation::Join { right_on, .. } => {
                    Some(right_on.len())
                }
                _ => None,
            }?;
            Some(derived.prepare(
                ryo_derived::domain::contract::types::ViewRequest::RawKeyOrder { stride },
                Vec::new(),
            ))
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let derived = super::super::derived::prepared::PreparedDerived::compile(&program, &operators)?;
    let mut metadata = Vec::with_capacity(plans.len());
    for (rule, plan) in plans.iter().enumerate() {
        let reads = crate::domain::trust::service::physical_reads(plan);
        let outputs = plan
            .observations
            .iter()
            .map(|output| RootMetadata {
                source: source(&program, named.as_ref(), rule, output.source),
                phase: output.scope,
                reads: match output.source {
                    Input::Step(step) => reads.get(step.0 as usize).cloned().unwrap_or_default(),
                    Input::Relation(relation) => vec![RelationRead {
                        relation,
                        kinds: vec![],
                    }],
                    Input::BoundObservation(_) => vec![],
                }
                .into_boxed_slice(),
            })
            .collect();
        metadata.push(RuleMetadata {
            emit: plan.emit.clone(),
            emit_root: program
                .rules()
                .get(rule)
                .ok_or(OperatingError::NotRun)?
                .emit
                .clone(),
            frontiers: plan.frontier.clone().into_boxed_slice(),
            observations: outputs,
            observation_frontiers: observation_frontiers[rule].clone().into_boxed_slice(),
            frontier_dependencies:
                crate::domain::file_program::service::observations::frontier_dependencies(plan)
                    .map_err(|error| OperatingError::ObservationIncomplete {
                        reason: error.to_string(),
                    }),
            semantic_needed: plan.steps.iter().any(|step| {
                matches!(
                    step.operation,
                    ryo_values::domain::plans::types::Operation::RenderSyntax { .. }
                )
            }),
        });
    }
    let matchers = Arc::new(
        crate::domain::matching::types::prepared::value::PreparedMatchers::from_operations(
            program.regions().iter().flat_map(|region| {
                region
                    .instructions
                    .iter()
                    .map(|instruction| &instruction.operation)
            }),
        ),
    );
    Ok(Arc::new(PreparedProgram {
        program,
        native_orders,
        rule_ids: rules.iter().map(|rule| rule.id.clone()).collect(),
        applicability: rules.iter().map(|rule| rule.applicability).collect(),
        rule_digests: rule_digests.to_vec().into_boxed_slice(),
        bindings: bindings.into_iter().map(Vec::into_boxed_slice).collect(),
        rules: metadata.into_boxed_slice(),
        observations: named,
        schedule,
        semantic_metadata: ryo_values::domain::run_metadata::service::from_plans(&plans),
        hosted: hosted.to_vec().into_boxed_slice(),
        matchers,
        derived,
        walk,
    }))
}
fn source(
    program: &ExecutableProgram,
    observations: Option<&ObservationMetadata>,
    rule: usize,
    source: Input,
) -> Result<InputSource, OperatingError> {
    match source {
        Input::Relation(relation) => Ok(InputSource::Relation(relation)),
        Input::Step(StepId(step)) => {
            let origin = program
                .rules()
                .get(rule)
                .and_then(|rule| rule.original_steps.get(step as usize))
                .ok_or(OperatingError::NotRun)?
                .as_ref()
                .map_err(Clone::clone)?;
            program
                .logical_regions()
                .get(origin.logical.0)
                .copied()
                .map(InputSource::Region)
                .ok_or(OperatingError::NotRun)
        }
        Input::BoundObservation(input) => observations
            .and_then(|observations| {
                observations
                    .slots
                    .iter()
                    .position(|slot| slot.rule == rule && slot.input == input)
            })
            .map(InputSource::Observation)
            .ok_or(OperatingError::NotRun),
    }
}

use ryo_values::domain::query_bindings::types::ParameterBinding;

use ryo_values::domain::rule_requests::types::RuleSpec;

/// Successful row delivery preserves physical uncertainty rather than fabricating an empty table.
pub enum RowEmission {
    /// No rows were delivered: the compiled root belongs to this later phase.
    Deferred {
        phase: StepScope,
    },
    Complete,
    Partial {
        affected: Vec<Option<ryo_relations::domain::relations::types::FileId>>,
    },
}
