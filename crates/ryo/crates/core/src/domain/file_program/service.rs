//! Typed request graph construction and bounded file-local successful result reuse.

pub(crate) mod link;
pub(crate) mod observations;

use super::types::{Node, NodeId, NodeInput, ObservationRequest, RequestGraph};

use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{Input, Operand, QueryLiteral, StepId};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Operation, Plan, StepScope};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

struct CanonicalNode {
    publication_reads: Vec<crate::domain::trust::types::RelationRead>,
    regions: Vec<super::types::PhysicalRegion>,
    needs: Result<
        Vec<crate::domain::trust::types::RelationRead>,
        crate::domain::operating::errors::OperatingError,
    >,
    contract: Arc<crate::domain::operator::contract::types::OperatorContract>,
    physical:
        crate::domain::table_model::collections::representations::publication::PreparedPhysical,
    scope: Option<usize>,
    phase: StepScope,
    input_supply: Vec<crate::domain::operating::types::Supply>,
    operation: Operation,
    inputs: Vec<NodeInput>,
    relations: Vec<Relation>,
    schema_valid: bool,
}

/// Original typed ancestry, prepared with the request. A boundary is an
/// admitted input of the physical region, never re-evaluated by that region.
fn region_ancestors(
    plan: &Plan,
    root: usize,
    stop: Option<usize>,
    mapped: &[Option<NodeId>],
) -> Vec<NodeId> {
    let mut pending = vec![root];
    let mut seen = HashSet::new();
    let mut nodes = Vec::new();
    while let Some(step) = pending.pop() {
        if !seen.insert(step) || Some(step) == stop {
            continue;
        }
        if step != root
            && let Some(Some(node)) = mapped.get(step)
        {
            nodes.push(*node);
        }
        if let Some(step) = plan.steps.get(step) {
            for input in named(&inputs_of(&step.operation)) {
                if let Input::Step(id) = input {
                    pending.push(id.0 as usize);
                }
            }
        }
    }
    nodes.sort_unstable_by_key(|id| id.0);
    nodes.dedup();
    nodes
}

fn node_input(
    input: Input,
    previous: &[Option<NodeId>],
    rule: usize,
    bound_offsets: &[usize],
) -> Option<(Input, NodeInput)> {
    match input {
        Input::Relation(relation) => Some((input, NodeInput::Relation(relation))),
        Input::BoundObservation(input) => Some((
            Input::Step(StepId(0)),
            NodeInput::Observation(super::types::ObservationGraphInputId(
                bound_offsets
                    .get(rule)?
                    .checked_add(usize::try_from(input.0).ok()?)?,
            )),
        )),
        Input::Step(step) => Some((
            Input::Step(StepId(0)),
            NodeInput::Node(*previous.get(step.0 as usize)?.as_ref()?),
        )),
    }
}

/// Preserve typed fields and ordered input edges; normalize only plan-local step
/// numbers. Operation equality itself ignores diagnostic Comparison.origin.
fn canonical(
    operation: &Operation,
    previous: &[Option<NodeId>],
    rule: usize,
    bound_offsets: &[usize],
) -> Option<(Operation, Vec<NodeInput>)> {
    let inputs = named(&inputs_of(operation))
        .iter()
        .map(|input| node_input(*input, previous, rule, bound_offsets))
        .collect::<Option<Vec<_>>>()?;
    let edges = inputs.iter().map(|(_, edge)| *edge).collect();
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
        | Operation::SpanBounds { from, .. } => *from = inputs[0].0,
        Operation::Join { left, right, .. }
        | Operation::Union { left, right }
        | Operation::Intersection { left, right }
        | Operation::Difference { left, right }
        | Operation::Antijoin { left, right, .. }
        | Operation::OrderedMatch { left, right, .. } => {
            *left = inputs[0].0;
            *right = inputs[1].0;
        }
        Operation::Axis {
            from,
            edges,
            domain,
            ..
        } => {
            *from = inputs[0].0;
            *edges = inputs[1].0;
            *domain = inputs[2].0;
        }
        Operation::Closure { edges, .. } | Operation::Cycles { edges, .. } => *edges = inputs[0].0,
        Operation::OrderedDfs { from, edges, .. } => {
            *from = inputs[0].0;
            *edges = inputs[1].0;
        }
        Operation::Rows { .. } | Operation::Hosted { .. } => {}
    }
    Some((normalized, edges))
}

fn materialized_steps(plan: &Plan) -> Vec<bool> {
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
    roots
        .into_iter()
        .zip(readers)
        .map(|(root, readers)| root || readers > 1)
        .collect()
}

/// Hash is an accelerator only: exact typed operation, edges and scope are compared.
fn fingerprint(
    operation: &Operation,
    inputs: &[NodeInput],
    scope: Option<usize>,
    phase: StepScope,
) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    std::mem::discriminant(operation).hash(&mut hash);
    inputs.hash(&mut hash);
    scope.hash(&mut hash);
    std::mem::discriminant(&phase).hash(&mut hash);
    hash.finish()
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
    let bound_offsets = if matches!(observations, ObservationRequest::Bound { .. }) {
        let mut offset = 0_usize;
        plans
            .iter()
            .map(|plan| {
                let start = offset;
                offset = offset
                    .checked_add(plan.observation_inputs.len())
                    .expect("request binding slots fit address space");
                start
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut all_mapped = Vec::with_capacity(
        if matches!(observations, ObservationRequest::Bound { .. }) {
            plans.len()
        } else {
            0
        },
    );
    let mut nodes = Vec::<CanonicalNode>::new();
    let mut cuts = Vec::with_capacity(plans.len());
    let mut observed_outputs = Vec::new();
    let mut buckets = HashMap::<u64, Vec<NodeId>>::new();
    for (rule, plan) in plans.iter().enumerate() {
        let kept = materialized_steps(plan);
        let publication_reads = crate::domain::trust::service::physical_reads(plan);
        let needs = crate::domain::trust::service::needs_of_contracts(plan, &operators.rules[rule]);
        let physical_plan = Arc::new(plan.clone());
        let layout = operators.rules[rule]
            .iter()
            .map(|contract| contract.collected_kinds.clone())
            .collect::<Result<Vec<_>, _>>()
            .ok()
            .map(|kinds| {
                Arc::new(crate::domain::operator::types::PlanLayout {
                    kept: crate::domain::operator::types::KeptSteps(kept.clone()),
                    kinds,
                    lowering: crate::domain::lowering::service::lower(plan),
                })
            });
        let columns = layout
            .as_ref()
            .map(|layout| crate::domain::column_kernel::service::compile(plan, layout));

        let mut mapped = Vec::with_capacity(plan.steps.len());
        let mut rule_cuts = Vec::with_capacity(plan.steps.len());
        for (index, step) in plan.steps.iter().enumerate() {
            let node = canonical(&step.operation, &mapped, rule, &bound_offsets).and_then(
                |(operation, inputs)| {
                    let contract = Arc::clone(operators.rules.get(rule)?.get(index)?);
                    let scope = (contract.reuse
                        == crate::domain::operator::contract::types::ReuseScope::Rule
                        || contract.effects
                            != crate::domain::operator::contract::types::Effects::Pure)
                        .then_some(rule);
                    let input_supply = Vec::new();
                    let region = layout.as_ref().and_then(|layout| {
                        let column = columns
                            .as_ref()
                            .and_then(|columns| columns.0.get(index))
                            .cloned()
                            .flatten();
                        let native = layout
                            .lowering
                            .shapes
                            .get(index)
                            .is_some_and(Option::is_some)
                            || layout.lowering.counting.get(index)
                                == Some(&crate::domain::lowering::types::Counting::Counts);
                        (native || column.is_some()).then(|| super::types::PhysicalRegion {
                            native_nodes: if native {
                                region_ancestors(plan, index, None, &mapped)
                            } else {
                                Vec::new()
                            },
                            column_nodes: column.as_ref().map_or_else(Vec::new, |chain| {
                                region_ancestors(plan, index, Some(chain.left_step), &mapped)
                            }),
                            plan: Arc::clone(&physical_plan),
                            layout: Arc::clone(layout),
                            rule,
                            step: StepId(index as u32),
                            column,
                        })
                    });
                    let hash = fingerprint(&operation, &inputs, scope, step.scope);
                    if let Some(found) = buckets.get(&hash).and_then(|bucket| {
                        bucket.iter().copied().find(|id| {
                            let existing = &nodes[id.0];
                            existing.operation == operation
                                && existing.inputs == inputs
                                && existing.scope == scope
                                && existing.phase == step.scope
                        })
                    }) {
                        if let Some(region) = region {
                            nodes[found.0].regions.push(region);
                        }
                        return Some(found);
                    }
                    let mut relations = Vec::new();
                    for input in &inputs {
                        let sources: &[Relation] = match input {
                            NodeInput::Relation(relation) => std::slice::from_ref(relation),
                            NodeInput::Node(id) => &nodes[id.0].relations,
                            NodeInput::Observation(_) => &[],
                        };
                        for relation in sources {
                            if !relations.contains(relation) {
                                relations.push(*relation);
                            }
                        }
                    }
                    let validated = contract.schema.clone().ok();
                    let schema_valid = validated.is_some()
                        && inputs.iter().all(|input| match input {
                            NodeInput::Node(id) => nodes[id.0].schema_valid,
                            NodeInput::Relation(_) | NodeInput::Observation(_) => true,
                        });
                    let id = NodeId(nodes.len());
                    nodes.push(CanonicalNode {
                        publication_reads: publication_reads
                            .get(index)
                            .cloned()
                            .unwrap_or_default(),
                        regions: region.into_iter().collect(),
                        needs: needs.as_ref().map_err(Clone::clone).and_then(|needs| {
                            needs
                                .get(index)
                                .cloned()
                                .ok_or(crate::domain::operating::errors::OperatingError::NotRun)
                        }),
                        physical: operators.physical[rule][index].clone(),
                        contract,
                        scope,
                        phase: step.scope,
                        input_supply,
                        operation,
                        inputs,
                        relations,
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
    // Only declared outputs are roots. Dependency traversal and all internal
    // last-consumer edges belong to the common graph, never an opaque schedule.
    let boundaries: Vec<Vec<(usize, NodeId)>> = plans
        .iter()
        .zip(&all_mapped)
        .map(|(plan, mapped)| {
            let mut roots = Vec::new();
            let mut add = |input: Input| {
                if let Input::Step(step) = input
                    && let Some(Some(node)) = mapped.get(step.0 as usize)
                    && !roots.iter().any(|(_, existing)| existing == node)
                {
                    roots.push((step.0 as usize, *node));
                }
            };
            add(Input::Step(plan.emit.step));
            for input in &plan.frontier {
                add(*input);
            }
            for output in &plan.observations {
                add(output.source);
            }
            roots
        })
        .collect();
    let rule_liveness = boundaries
        .iter()
        .map(|roots| {
            let mut counts = HashMap::<NodeId, usize>::new();
            let mut expanded = HashSet::new();
            let mut pending = Vec::new();
            for (_, root) in roots {
                *counts.entry(*root).or_default() += 1;
                pending.push(*root);
            }
            while let Some(node) = pending.pop() {
                if !expanded.insert(node) {
                    continue;
                }
                for input in &nodes[node.0].inputs {
                    if let NodeInput::Node(child) = input {
                        *counts.entry(*child).or_default() += 1;
                        pending.push(*child);
                    }
                }
            }
            let mut counts = counts.into_iter().collect::<Vec<_>>();
            counts.sort_unstable_by_key(|(node, _)| std::cmp::Reverse(node.0));
            counts
        })
        .collect();
    let mut derived_sources = vec![false; nodes.len()];
    for (rule, plan) in plans.iter().enumerate() {
        for (step, operation) in plan.steps.iter().enumerate() {
            let Some(contract) = operators
                .rules
                .get(rule)
                .and_then(|contracts| contracts.get(step))
            else {
                continue;
            };
            let inputs = inputs_of(&operation.operation);
            for group in contract
                .derived
                .streaming
                .iter()
                .chain(&contract.derived.collected)
                .flatten()
            {
                for use_ in std::iter::once(&group.primary).chain(group.secondary.iter()) {
                    for place in &use_.inputs[..usize::from(use_.len)] {
                        if let Some(Input::Step(id)) = named(&inputs).get(*place)
                            && let Some(Some(node)) =
                                cuts.get(rule).and_then(|steps| steps.get(id.0 as usize))
                        {
                            derived_sources[node.0] = true;
                        }
                    }
                }
            }
        }
    }
    // Canonical sharing can add consumers absent from any original-plan use
    // count. Include terminal/frontier observations, even where cuts are None.
    let mut resolved_readers = vec![0usize; nodes.len()];
    for node in &nodes {
        for input in &node.inputs {
            if let NodeInput::Node(id) = input {
                resolved_readers[id.0] += 1;
            }
        }
    }
    for root in observed_outputs {
        resolved_readers[root.0] += 1;
    }
    let mut canonical_roots = vec![false; nodes.len()];
    for roots in &boundaries {
        for (_, root) in roots {
            canonical_roots[root.0] = true;
        }
    }
    for node in &mut nodes {
        node.input_supply = node
            .inputs
            .iter()
            .map(|input| match input {
                NodeInput::Node(child)
                    if node.phase == StepScope::PerFile
                        && resolved_readers[child.0] == 1
                        && !canonical_roots[child.0] =>
                {
                    crate::domain::operating::types::Supply::Streamed
                }
                _ => crate::domain::operating::types::Supply::AtHand,
            })
            .collect();
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
                ordered_existence[select.0] = Some(
                    crate::domain::ordered_existence::types::ResolvedAlternative::Declined(
                        crate::domain::ordered_existence::types::Decline::ExtraConsumer,
                    ),
                );
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

            ordered_existence[select.0] = Some(
                crate::domain::ordered_existence::types::ResolvedAlternative::Eligible(
                    crate::domain::ordered_existence::types::ResolvedShape {
                        shape: shape.clone(),
                        source,
                        join,
                        select,
                        output: output.clone(),
                    },
                ),
            );
        }
    }
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
        rule_liveness,
        boundaries,
        nodes: nodes
            .into_iter()
            .map(|node| Node {
                publication_reads: node.publication_reads,
                regions: node.regions,
                needs: node.needs,
                phase: node.phase,
                input_supply: node.input_supply,
                contract: node.contract,
                physical: node.physical,
                operation: node.operation,
                inputs: node.inputs,
                schema_valid: node.schema_valid,
                relations: node.relations,
            })
            .collect(),
        cuts,
    }
}

#[cfg(test)]
mod service_test;
