//! Immutable request preparation, physical metadata and compilation ownership.
use super::types::*;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::executable_program::types::ExecutableProgram;
use crate::domain::executable_program::types::InputSource;
use crate::domain::file_program::types::ObservedFrontierRoot;
use crate::domain::file_program::types::RuleSchedule;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::RequestOperators;
use crate::domain::prepared_request::types::Compiled;
use crate::domain::prepared_request::types::ObservationIdentity;
use crate::domain::source_partitions::types::source_partitions::value::SourcePartitions;
use crate::domain::trust::types::RelationRead;
use crate::outbound::derived::interface::DerivedPort;

use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::observations::types::ObservationReadinessLayout;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::plans::types::StepScope;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rule_requests::types::RuleSpec;
use std::sync::Arc;

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
        input: usize,
    ) -> Result<&[usize], OperatingError> {
        self.file_frontier_outputs
            .get(rule.0)
            .and_then(|inputs| inputs.get(input))
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
    ) -> &crate::application::file_dag::derived::prepared::PreparedDerived {
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
    pub(crate) fn metadata(&self) -> &[ryo_values::domain::run_metadata::types::RuleRunMetadata] {
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
        self.semantic_needed
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
    if tracing::enabled!(target: crate::application::file_dag::program::constants::WALK_TARGET, tracing::Level::TRACE)
    {
        let census = walk.census();
        tracing::trace!(target: crate::application::file_dag::program::constants::WALK_TARGET, event = crate::application::file_dag::program::constants::WALK_CENSUS,
            rules = ?rules.iter().map(|rule| rule.id.0.as_ref()).collect::<Vec<_>>(),
            instructions = census.instructions.0,
            physical_instructions = program.regions().iter().map(|region| region.instructions.len()).sum::<usize>(),
            shared = census.shared.0,
            classes = census.classes.len(), representation = census.representation.0,
            dependent = census.dependent.0, observation = census.observation.0,
            row = census.row.0, derived = census.derived.0, request = census.request.0,
            continuation = census.continuation.0);
        for class in census.classes {
            tracing::trace!(target: crate::application::file_dag::program::constants::WALK_TARGET, event = crate::application::file_dag::program::constants::WALK_CLASS,
                class = class.0, relation = ?walk.classes()[class.0].source,
                heads = ?program.regions().iter().flat_map(|region| region.instructions.iter()).filter(|instruction| walk.class_for(instruction) == Some(class)).map(|instruction| instruction.logical.0).collect::<Vec<_>>());
        }
    }

    let native_orders = program
        .regions()
        .iter()
        .map(|region| {
            if region.native_positions.is_none() {
                return Ok(None);
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
            };
            stride
                .map(|stride| {
                    crate::application::file_dag::derived::prepare::view(
                        Arc::clone(&derived),
                        &crate::domain::derived_views::types::ViewRequest::RawKeyOrder { stride },
                        &[],
                    )
                    .map(Arc::new)
                })
                .transpose()
        })
        .collect::<Result<Vec<_>, OperatingError>>()?
        .into_boxed_slice();
    let derived = crate::application::file_dag::derived::prepared::PreparedDerived::compile(
        &program, &operators,
    )?;
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
    let file_demands = metadata
        .iter()
        .enumerate()
        .map(|(rule, metadata)| {
            let outputs = metadata
                .observations
                .iter()
                .map(|root| {
                    crate::domain::executable_program::file_demand::types::ObservationDemandSource {
                        source: root.source.clone(),
                        phase: root.phase,
                    }
                })
                .collect::<Vec<_>>();
            let frontiers = metadata
                .observation_frontiers
                .iter()
                .map(|observed| {
                    use crate::domain::executable_program::file_demand::types::FrontierPurpose;
                    if *observed {
                        FrontierPurpose::Observation
                    } else {
                        FrontierPurpose::Diagnostic
                    }
                })
                .collect::<Vec<_>>();
            crate::domain::executable_program::file_demand::service::compile(
                &program,
                RuleIndex(rule),
                &outputs,
                &frontiers,
            )
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_boxed_slice();
    let file_frontier_outputs = metadata
        .iter()
        .enumerate()
        .map(|(rule, metadata)| {
            (0..metadata.frontiers.len())
                .map(|input| {
                    named
                        .iter()
                        .flat_map(|named| named.frontiers.iter())
                        .filter(|root| root.producer.0 == rule && root.frontier.0 == input)
                        .map(|root| root.output.0 as usize)
                        .collect::<Vec<_>>()
                        .into_boxed_slice()
                })
                .collect::<Vec<_>>()
                .into_boxed_slice()
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let semantic_needed = metadata.iter().any(|rule| rule.semantic_needed);
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
