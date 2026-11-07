//! Typed request graph construction and bounded file-local successful result reuse.

pub(crate) mod link;
pub(crate) mod observations;

use super::constants::{EXPECT_BINDING_SLOTS, ORDERED_EXISTENCE_DECLINED, PREPARATION_TARGET};
use super::types::{
    BindingOffset, CanonicalOperation, DerivedSources, FileProgramNode, Fingerprint, NodeId,
    NodeInput, ObservationRequest, RequestGraph, ResolvedAlternative, ResolvedInput,
    ResolvedShape,
};
use crate::domain::coverage::types::RuleIndex;
use crate::domain::operator::contract::types::RequestOperators;
use crate::domain::operator::types::KeptSteps;
use crate::domain::self_join_recogniser::types::Decline;

use ryo_values::domain::plan_parts::service::{binding_place, step_in};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{Input, Operand, QueryLiteral, StepId};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Operation, Plan};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

fn node_input(
    input: Input,
    previous: &[Option<NodeId>],
    rule: RuleIndex,
    bound_offsets: &[BindingOffset],
) -> Option<ResolvedInput> {
    match input {
        Input::Relation(relation) => Some(ResolvedInput {
            input,
            edge: NodeInput::Relation(relation),
        }),
        Input::BoundObservation(input) => Some(ResolvedInput {
            input: Input::Step(StepId(0)),
            edge: NodeInput::Observation(super::types::ObservationGraphInputId(
                bound_offsets
                    .get(rule.0)?
                    .0
                    .checked_add(binding_place(input))?,
            )),
        }),
        Input::Step(step) => Some(ResolvedInput {
            input: Input::Step(StepId(0)),
            edge: NodeInput::Node(*step_in(previous, step)?.as_ref()?),
        }),
    }
}

/// Preserve typed fields and ordered input edges; normalize only plan-local step
/// numbers. Operation equality itself ignores diagnostic Comparison.origin.
fn canonical(
    operation: &Operation,
    previous: &[Option<NodeId>],
    rule: RuleIndex,
    bound_offsets: &[BindingOffset],
) -> Option<CanonicalOperation> {
    let inputs = named(&inputs_of(operation))
        .iter()
        .map(|input| node_input(*input, previous, rule, bound_offsets))
        .collect::<Option<Vec<_>>>()?;
    let edges = inputs.iter().map(|resolved| resolved.edge).collect();
    let mut normalized = operation.clone();
    match &mut normalized {
        Operation::Select { from, .. }
        | Operation::Project { from, .. }
        | Operation::Extend { from, .. }
        | Operation::GroupCount { from, .. }
        | Operation::GroupDigest { from, .. }
        | Operation::Capture { from, .. }
        | Operation::Rank { from, .. }
        | Operation::Replace { from, .. }
        | Operation::Words { from, .. }
        | Operation::SpanBetween { from, .. }
        | Operation::SourceMatches { from, .. }
        | Operation::RenderSyntax { from, .. }
        | Operation::FitsRendered { from, .. }
        | Operation::SpanBounds { from, .. } => *from = inputs[0].input,
        Operation::Join { left, right, .. }
        | Operation::Union { left, right }
        | Operation::Intersection { left, right }
        | Operation::Difference { left, right }
        | Operation::Antijoin { left, right, .. }
        | Operation::OrderedMatch { left, right, .. } => {
            *left = inputs[0].input;
            *right = inputs[1].input;
        }
        Operation::Axis {
            from,
            edges,
            domain,
            ..
        } => {
            *from = inputs[0].input;
            *edges = inputs[1].input;
            *domain = inputs[2].input;
        }
        Operation::Closure { edges, .. } | Operation::Cycles { edges, .. } => {
            *edges = inputs[0].input
        }
        Operation::OrderedDfs { from, edges, .. } => {
            *from = inputs[0].input;
            *edges = inputs[1].input;
        }
        Operation::Rows { .. } | Operation::Hosted { .. } => {}
    }
    Some(CanonicalOperation {
        operation: normalized,
        inputs: edges,
    })
}

pub(crate) fn materialized_steps(plan: &Plan) -> KeptSteps {
    let mut readers = vec![0usize; plan.steps.len()];
    for step in &plan.steps {
        if step.scope != StepScope::PerFile {
            continue;
        }
        for input in named(&inputs_of(&step.operation)) {
            if let Input::Step(id) = input
                && let Some(count) = readers.get_mut(id.0 as usize)
            {
                *count += 1;
            }
        }
    }
    let mut roots = vec![false; plan.steps.len()];
    for input in plan
        .frontier
        .iter()
        .copied()
        .chain(std::iter::once(Input::Step(plan.emit.step)))
        .chain(plan.observations.iter().map(|root| root.source))
    {
        if let Input::Step(id) = input
            && let Some(root) = roots.get_mut(id.0 as usize)
        {
            *root = true;
        }
    }
    KeptSteps(
        roots
            .into_iter()
            .zip(readers)
            .map(|(root, readers)| root || readers > 1)
            .collect(),
    )
}

/// Hash is an accelerator only: exact typed operation, edges and scope are compared.
fn fingerprint(
    operation: &Operation,
    inputs: &[NodeInput],
    scope: Option<RuleIndex>,
    phase: StepScope,
) -> Fingerprint {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    std::mem::discriminant(operation).hash(&mut hash);
    inputs.hash(&mut hash);
    scope.hash(&mut hash);
    std::mem::discriminant(&phase).hash(&mut hash);
    Fingerprint(hash.finish())
}

/// Compile one DAG for the entire ordered request. Unsupported descendants
/// break the chain; no raw Debug serialization or source-origin text is a key.
#[cfg(test)]
pub(crate) fn compile_prepared(
    plans: &[Plan],
    operators: &crate::domain::operator::contract::types::RequestOperators,
) -> RequestGraph {
    compile_prepared_observations(plans, operators, ObservationRequest::Independent)
}

/// Extend the same graph with named producer edges after all real contracts are prepared.
pub(crate) fn compile_prepared_observations(
    plans: &[Plan],
    operators: &crate::domain::operator::contract::types::RequestOperators,
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
    let mut buckets = HashMap::<Fingerprint, Vec<NodeId>>::new();
    for (rule, plan) in plans.iter().enumerate() {
        let kept = materialized_steps(plan);
        let rule_index = RuleIndex(rule);
        let needs = crate::domain::trust::service::needs_of_contracts(plan, &operators.rules[rule]);
        // The one analysis of the rule: the lowering is made once and the column chains are
        // compiled over it. Nothing of it outlives the loop but the shapes found per step.
        let layout = match operators.rules[rule]
            .iter()
            .map(|contract| contract.output_kinds.clone())
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(kinds) => Some(crate::domain::operator::types::PlanLayout {
                kept: KeptSteps(kept.0.clone()),
                kinds,
                lowering: crate::domain::lowering::service::lower(plan),
            }),
            Err(_) => None,
        };
        let columns = layout
            .as_ref()
            .map(|layout| crate::domain::column_kernel::service::compile(plan, layout));

        let mut mapped = Vec::with_capacity(plan.steps.len());
        let mut rule_cuts = Vec::with_capacity(plan.steps.len());
        for (index, step) in plan.steps.iter().enumerate() {
            let node = canonical(&step.operation, &mapped, rule_index, &bound_offsets).and_then(
                |CanonicalOperation { operation, inputs }| {
                    let contract = Arc::clone(operators.rules.get(rule)?.get(index)?);
                    let scope = (contract.reuse
                        == crate::domain::operator::contract::types::ReuseScope::Rule
                        || contract.effects
                            != crate::domain::operator::contract::types::Effects::Pure)
                        .then_some(rule_index);
                    let region = layout.as_ref().and_then(|layout| {
                        let column = columns
                            .as_ref()
                            .and_then(|columns| columns.0.get(index))
                            .cloned()
                            .flatten();
                        // A step that only counts the rows of a lowered step makes no positions.
                        let native = (step.scope == StepScope::PerFile
                            && layout.lowering.counting.get(index)
                                == Some(&crate::domain::lowering::types::Counting::DoesNotCount))
                        .then(|| layout.lowering.shapes.get(index).and_then(Option::as_ref))
                        .flatten()
                        .map(|shape| Arc::new(shape.clone()));
                        (native.is_some() || column.is_some()).then(|| super::types::StepAnalysis {
                            rule: rule_index,
                            step: StepId(index as u32),
                            native,
                            column,
                        })
                    });
                    let hash = fingerprint(&operation, &inputs, scope, step.scope);
                    // Delivery is outside semantic evaluation. Identical pure
                    // terminals and intermediates share one canonical publication;
                    // authored root-reader obligations remain distinct below.
                    if let Some(found) = buckets.get(&hash).and_then(|bucket| {
                        bucket.iter().copied().find(|id| {
                            let existing = &nodes[id.0];
                            existing.operation == operation
                                && existing.inputs == inputs
                                && scopes[id.0] == scope
                                && existing.phase == step.scope
                        })
                    }) {
                        if let Some(region) = region {
                            nodes[found.0].analysis.push(region);
                        }
                        return Some(found);
                    }
                    let schema_valid = contract.schema.is_ok()
                        && inputs.iter().all(|input| match input {
                            NodeInput::Node(id) => nodes[id.0].schema_valid,
                            NodeInput::Relation(_) | NodeInput::Observation(_) => true,
                        });
                    let id = NodeId(nodes.len());
                    scopes.push(scope);
                    nodes.push(FileProgramNode {
                        analysis: region.into_iter().collect(),
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
        derived_sources: derived_sources.0,
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
    cuts: &[Vec<Option<NodeId>>],
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
    cuts: &[Vec<Option<NodeId>>],
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
            for place in &use_.inputs[..usize::from(use_.len)] {
                if let Some(Input::Step(id)) = named(&inputs).get(*place)
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
    cuts: &[Vec<Option<NodeId>>],
    nodes: &[FileProgramNode],
    observed_outputs: &[NodeId],
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
    for (rule, program) in operators.ordered_existence.iter().enumerate() {
        let Some(plan) = plans.get(rule) else {
            continue;
        };
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
