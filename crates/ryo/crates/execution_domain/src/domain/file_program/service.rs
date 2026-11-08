//! Typed request graph construction and bounded file-local successful result reuse.

pub mod link;
pub mod observations;
pub mod operators;

use super::constants::{EXPECT_BINDING_SLOTS, ORDERED_EXISTENCE_DECLINED, PREPARATION_TARGET};
use super::types::canonical_operation::value::CanonicalOperation;
use super::types::fingerprint::value::Fingerprint;
use super::types::{
    BindingOffset, DerivedSources, FileProgramNode, ProgramNodeId, NodeInput, ObservationRequest,
    RequestGraph, ResolvedAlternative, ResolvedShape, SchemaValidity,
};
use crate::domain::coverage::types::RuleIndex;
use crate::domain::file_program::types::RequestOperators;
use crate::domain::self_join_recogniser::types::Decline;

use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{Input, StepId};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Operation, Plan};
use std::collections::HashMap;
use std::sync::Arc;


/// Compile one DAG for the entire ordered request. Unsupported descendants
/// break the chain; no raw Debug serialization or source-origin text is a key.
#[cfg(any(test, feature = "fixtures"))]
pub fn compile_prepared(
    plans: &[Plan],
    operators: &RequestOperators,
) -> RequestGraph {
    compile_prepared_observations(plans, operators, ObservationRequest::Independent)
}

/// Extend the same graph with named producer edges after all real contracts are prepared.
pub fn compile_prepared_observations(
    plans: &[Plan],
    operators: &RequestOperators,
    observations: ObservationRequest<'_>,
) -> RequestGraph {
    let bound_offsets = binding_offsets(plans, observations);
    let mut all_mapped = Vec::with_capacity(
        if matches!(observations, ObservationRequest::Bound { .. }) {
            plans.len()
        } else {
            0
        },
    );
    let mut nodes = Vec::<FileProgramNode>::new();
    let mut scopes = Vec::<Option<RuleIndex>>::new();
    let mut cuts = Vec::with_capacity(plans.len());
    let mut observed_outputs = Vec::new();
    let mut buckets = HashMap::<Fingerprint, Vec<ProgramNodeId>>::new();
    for (rule, plan) in plans.iter().enumerate() {
        let rule_index = RuleIndex(rule);
        let needs = crate::domain::trust::service::needs_of_contracts(plan, &operators.rules[rule]);
        let mut mapped = Vec::with_capacity(plan.steps.len());
        let mut rule_cuts = Vec::with_capacity(plan.steps.len());
        for (index, step) in plan.steps.iter().enumerate() {
            let node = CanonicalOperation::of(&step.operation, &mapped, rule_index, &bound_offsets).and_then(
                |CanonicalOperation { operation, inputs }| {
                    let contract = Arc::clone(operators.rules.get(rule)?.get(index)?);
                    let scope = (contract.reuse
                        == crate::domain::operator::contract::types::ReuseScope::Rule)
                        .then_some(rule_index);
                    let hash = Fingerprint::of(&operation, &inputs, scope, step.scope);
                    // Delivery is outside semantic evaluation. Equal terminals and
                    // intermediates in one reuse scope share one canonical publication:
                    // request-wide for any effect but a request source when nothing is
                    // bound, else the rule's. Authored root-reader obligations remain
                    // distinct below.
                    if let Some(found) = buckets.get(&hash).and_then(|bucket| {
                        bucket.iter().copied().find(|id| {
                            let existing = &nodes[id.0];
                            existing.operation == operation
                                && existing.inputs == inputs
                                && scopes[id.0] == scope
                                && existing.phase == step.scope
                        })
                    }) {
                        return Some(found);
                    }
                    let schema_valid = if contract.schema.is_ok()
                        && inputs.iter().all(|input| match input {
                            NodeInput::Node(id) => nodes[id.0].schema_valid == SchemaValidity::Valid,
                            NodeInput::Relation(_) | NodeInput::Observation(_) => true,
                        }) {
                        SchemaValidity::Valid
                    } else {
                        SchemaValidity::Invalid
                    };
                    let id = ProgramNodeId(nodes.len());
                    scopes.push(scope);
                    nodes.push(FileProgramNode {
                        needs: needs.as_ref().map_err(Clone::clone).and_then(|needs| {
                            needs
                                .get(index)
                                .cloned()
                                .ok_or(crate::domain::operating::errors::OperatingError::NotRun)
                        }),
                        contract,
                        phase: step.scope,
                        operation,
                        inputs,
                        schema_valid,
                    });
                    buckets.entry(hash).or_default().push(id);
                    Some(id)
                },
            );
            let exported = plan
                .observations
                .iter()
                .any(|output| output.source == Input::Step(StepId(index as u32)));
            let internal = exported
                || (index != plan.emit.step.0 as usize
                    && !plan.frontier.contains(&Input::Step(StepId(index as u32))));
            if let Some(node) = node {
                if !internal || exported {
                    observed_outputs.push(node);
                }
            }
            rule_cuts.push(node.filter(|_| internal && step.scope == StepScope::PerFile));
            mapped.push(node);
        }
        all_mapped.push(mapped);
        cuts.push(rule_cuts);
    }
    let derived_sources = derived_sources_of(plans, operators, &cuts, &nodes);
    let ordered_existence =
        ordered_alternatives(plans, operators, &cuts, &nodes, &observed_outputs);
    let observations = observations::graph_outputs(plans, &all_mapped, observations);
    let program_nodes = all_mapped
        .iter()
        .map(|steps| {
            steps
                .iter()
                .map(|node| node.ok_or(crate::domain::operating::errors::OperatingError::NotRun))
                .collect()
        })
        .collect();
    RequestGraph {
        program_nodes,
        ordered_existence,
        observations,
        derived_sources,
        nodes,
    }
}

/// Where the binding slots of each rule start among the slots of the request, when it binds named roots.
fn binding_offsets(plans: &[Plan], observations: ObservationRequest<'_>) -> Vec<BindingOffset> {
    if !matches!(observations, ObservationRequest::Bound { .. }) {
        return Vec::new();
    }
    let mut offset = 0_usize;
    plans
        .iter()
        .map(|plan| {
            let start = offset;
            offset = offset
                .checked_add(plan.observation_inputs.len())
                .expect(EXPECT_BINDING_SLOTS);
            BindingOffset(start)
        })
        .collect()
}

/// Producers of declared derived inputs, computed once for the whole request.
fn derived_sources_of(
    plans: &[Plan],
    operators: &RequestOperators,
    cuts: &[Vec<Option<ProgramNodeId>>],
    nodes: &[FileProgramNode],
) -> DerivedSources {
    let mut found = DerivedSources(vec![false; nodes.len()]);
    for (rule, plan) in plans.iter().enumerate() {
        plan_sources(RuleIndex(rule), plan, operators, cuts, &mut found);
    }
    found
}

/// Mark the nodes that the derived views of one plan's steps read.
fn plan_sources(
    rule: RuleIndex,
    plan: &Plan,
    operators: &RequestOperators,
    cuts: &[Vec<Option<ProgramNodeId>>],
    found: &mut DerivedSources,
) {
    for (step, operation) in plan.steps.iter().enumerate() {
        let Some(contract) = operators
            .rules
            .get(rule.0)
            .and_then(|contracts| contracts.get(step))
        else {
            continue;
        };
        let inputs = inputs_of(&operation.operation);
        let uses = contract
            .derived
            .slots
            .iter()
            .flatten()
            .flat_map(|group| std::iter::once(&group.primary).chain(group.secondary.iter()));
        for use_ in uses {
            for place in &use_.inputs[..usize::from(use_.len.0)] {
                if let Some(Input::Step(id)) = named(&inputs).get(place.0)
                    && let Some(Some(node)) =
                        cuts.get(rule.0).and_then(|steps| steps.get(id.0 as usize))
                {
                    found.0[node.0] = true;
                }
            }
        }
    }
}

/// Reviewed alternatives indexed by their replaced Select producer.
fn ordered_alternatives(
    plans: &[Plan],
    operators: &RequestOperators,
    cuts: &[Vec<Option<ProgramNodeId>>],
    nodes: &[FileProgramNode],
    observed_outputs: &[ProgramNodeId],
) -> Vec<Option<ResolvedAlternative>> {
    // Canonical sharing can add consumers absent from any original-plan use
    // count. Include terminal/frontier observations, even where cuts are None.
    let mut resolved_readers = vec![0usize; nodes.len()];
    for node in nodes {
        for input in &node.inputs {
            if let NodeInput::Node(id) = input {
                resolved_readers[id.0] += 1;
            }
        }
    }
    for root in observed_outputs {
        resolved_readers[root.0] += 1;
    }
    let mut ordered_existence = vec![None; nodes.len()];
    for (rule, plan) in plans.iter().enumerate() {
        // Only a rule with an anti-join can hold the shape; its program is compiled the first
        // time its regions are resolved, never for every rule of the request.
        if !plan
            .steps
            .iter()
            .any(|step| matches!(step.operation, Operation::Antijoin { .. }))
        {
            continue;
        }
        let Some(slot) = operators.ordered_existence.get(rule) else {
            continue;
        };
        let program = slot.get_or_init(|| {
            crate::domain::self_join_recogniser::service::compile(plan, &|id| {
                match &operators.rules.get(rule)?.get(id.0 as usize)?.schema {
                    Ok(kinds) => Some(kinds.as_slice()),
                    Err(_) => None,
                }
            })
        });
        for shape in &program.shapes {
            if shape.plan.0 != std::ptr::from_ref(plan).addr() {
                continue;
            }
            let resolve = |step: StepId| cuts.get(rule)?.get(step.0 as usize).copied().flatten();
            let (Some(source), Some(join), Some(select), Some(project)) = (
                resolve(shape.source),
                resolve(shape.replaced_join),
                resolve(shape.replaced_select),
                resolve(shape.retained_project),
            ) else {
                continue;
            };
            if [join, select, project]
                .iter()
                .any(|id| resolved_readers[id.0] != 1)
            {
                ordered_existence[select.0] =
                    Some(ResolvedAlternative::Declined(Decline::ExtraConsumer));
                continue;
            }
            // Verify canonical edges, not normalized operation-local Step numbers.
            if nodes[join.0].inputs != [NodeInput::Node(source), NodeInput::Node(source)]
                || nodes[select.0].inputs != [NodeInput::Node(join)]
            {
                continue;
            }
            let Operation::Join { output, .. } =
                &plan.steps[shape.replaced_join.0 as usize].operation
            else {
                continue;
            };
            let Operation::Antijoin {
                right: Input::Step(project_input),
                ..
            } = &plan.steps[shape.retained_antijoin.0 as usize].operation
            else {
                continue;
            };
            if *project_input != shape.retained_project {
                continue;
            }
            let Operation::Project { columns, .. } =
                &plan.steps[shape.retained_project.0 as usize].operation
            else {
                continue;
            };
            let projected_left = columns
                .iter()
                .map(|column| output.get(column.0 as usize).map(|mapped| mapped.column))
                .collect::<Option<Vec<_>>>();
            if projected_left.as_deref() != Some(shape.projected_left.as_slice()) {
                continue;
            }

            ordered_existence[select.0] = Some(ResolvedAlternative::Eligible(ResolvedShape {
                shape: shape.clone(),
                source,
                join,
                select,
                output: output.clone(),
            }));
        }
    }
    for (node, alternative) in ordered_existence.iter().enumerate() {
        if let Some(ResolvedAlternative::Declined(reason)) = alternative {
            tracing::trace!(target: PREPARATION_TARGET, event = ORDERED_EXISTENCE_DECLINED, node, ?reason);
        }
    }
    ordered_existence
}
