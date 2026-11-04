//! Typed request graph construction and bounded file-local successful result reuse.

use super::types::{FileProgram, Node, NodeId, NodeInput, RequestGraph};

use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{Input, Operand, QueryLiteral, StepId};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Operation, Plan, StepScope};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

// One small and one large table bound this opt-in owner to at most six MiB of
// Value buffers. A small early cut cannot occupy the slot needed by an observed
// 262,144-cell shared Join. No per-allocation heap ledger or index-cache growth.
const SMALL_RESULT_CELL_CAPACITY: usize = 128 * 1024;
const LARGE_RESULT_CELL_CAPACITY: usize = 256 * 1024;

struct CanonicalNode {
    contract: Arc<crate::domain::operator::contract::types::OperatorContract>,
    physical:
        crate::domain::table_model::collections::representations::publication::PreparedPhysical,
    scope: Option<usize>,
    operation: Operation,
    inputs: Vec<NodeInput>,
    relations: Vec<Relation>,
    schema_valid: bool,
}

fn node_input(input: Input, previous: &[Option<NodeId>]) -> Option<(Input, NodeInput)> {
    match input {
        Input::Relation(relation) => Some((input, NodeInput::Relation(relation))),
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
) -> Option<(Operation, Vec<NodeInput>)> {
    let inputs = named(&inputs_of(operation))
        .iter()
        .map(|input| node_input(*input, previous))
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

/// Hash is an accelerator only: exact typed operation, edges and scope are compared.
fn fingerprint(operation: &Operation, inputs: &[NodeInput], scope: Option<usize>) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    std::mem::discriminant(operation).hash(&mut hash);
    inputs.hash(&mut hash);
    scope.hash(&mut hash);
    hash.finish()
}

/// Compile one DAG for the entire ordered request. Unsupported descendants
/// break the chain; no raw Debug serialization or source-origin text is a key.
pub(crate) fn compile_prepared(
    plans: &[Plan],
    operators: &crate::domain::operator::contract::types::RequestOperators,
) -> RequestGraph {
    let mut nodes = Vec::<CanonicalNode>::new();
    let mut cuts = Vec::with_capacity(plans.len());
    let mut buckets = HashMap::<u64, Vec<NodeId>>::new();
    for (rule, plan) in plans.iter().enumerate() {
        let mut mapped = Vec::with_capacity(plan.steps.len());
        let mut rule_cuts = Vec::with_capacity(plan.steps.len());
        for (index, step) in plan.steps.iter().enumerate() {
            let node = if step.scope == StepScope::PerFile {
                canonical(&step.operation, &mapped).and_then(|(operation, inputs)| {
                    let contract = Arc::clone(operators.rules.get(rule)?.get(index)?);
                    let scope = (contract.reuse
                        == crate::domain::operator::contract::types::ReuseScope::Rule
                        || contract.effects
                            != crate::domain::operator::contract::types::Effects::Pure)
                        .then_some(rule);
                    let hash = fingerprint(&operation, &inputs, scope);
                    if let Some(found) = buckets.get(&hash).and_then(|bucket| {
                        bucket.iter().copied().find(|id| {
                            let existing = &nodes[id.0];
                            existing.operation == operation
                                && existing.inputs == inputs
                                && existing.scope == scope
                        })
                    }) {
                        return Some(found);
                    }
                    let mut relations = Vec::new();
                    for input in &inputs {
                        let sources: &[Relation] = match input {
                            NodeInput::Relation(relation) => std::slice::from_ref(relation),
                            NodeInput::Node(id) => &nodes[id.0].relations,
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
                            NodeInput::Relation(_) => true,
                        });
                    let id = NodeId(nodes.len());
                    nodes.push(CanonicalNode {
                        physical: operators.physical[rule][index].clone(),
                        contract,
                        scope,
                        operation,
                        inputs,
                        relations,
                        schema_valid,
                    });
                    buckets.entry(hash).or_default().push(id);
                    Some(id)
                })
            } else {
                None
            };
            let internal = index != plan.emit.step.0 as usize
                && !plan.frontier.contains(&Input::Step(StepId(index as u32)));
            rule_cuts.push(node.filter(|_| internal));
            mapped.push(node);
        }
        cuts.push(rule_cuts);
    }
    // Every cut occurrence remains available to the executor, but lifetime
    // accounting visits each rule/node pair once, without file-local dedup work.
    let mut seen = HashSet::new();
    let consumers = cuts
        .iter()
        .map(|cuts| {
            seen.clear();
            cuts.iter()
                .flatten()
                .copied()
                .filter(|node| seen.insert(*node))
                .collect()
        })
        .collect();
    let boundaries: Vec<Vec<(usize, NodeId)>> = plans
        .iter()
        .zip(&cuts)
        .map(|(plan, owned)| {
            owned
                .iter()
                .enumerate()
                .filter_map(|(index, node)| {
                    let node = (*node)?;
                    let input = Input::Step(StepId(index as u32));
                    plan.steps
                        .iter()
                        .enumerate()
                        .any(|(consumer, step)| {
                            owned.get(consumer).is_none_or(Option::is_none)
                                && named(&inputs_of(&step.operation)).contains(&input)
                        })
                        .then_some((index, node))
                })
                .collect()
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
    RequestGraph {
        derived_sources,
        rule_liveness,
        boundaries,
        consumers,
        plans: plans
            .iter()
            .map(|plan| std::ptr::from_ref(plan).addr())
            .collect(),
        nodes: nodes
            .into_iter()
            .map(|node| Node {
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

impl FileProgram {
    pub(crate) fn cut(&self, rule: usize, step: usize) -> Option<NodeId> {
        let id = self
            .graph
            .cuts
            .get(rule)
            .and_then(|cuts| cuts.get(step))
            .copied()
            .flatten();
        let remaining = id
            .and_then(|id| self.remaining.get(id.0))
            .copied()
            .unwrap_or(0);
        let retained = id.is_some_and(|id| self.entries.iter().any(|entry| entry.0 == id));
        let admitted = retained || remaining > 1;
        tracing::trace!(target: "ryo::diagnostic::file_program", event = "cut_decision",
            rule, step, node = ?id, remaining, retained, admitted);
        admitted.then_some(id).flatten()
    }

    pub(crate) fn get(&mut self, node: NodeId) -> Option<Arc<Table>> {
        let table = Arc::clone(&self.entries.iter().find(|entry| entry.0 == node)?.1);
        self.shared_executions += 1;
        tracing::trace!(target: "ryo::diagnostic::file_program", event = "hit",
            node = node.0, cells = table.cells.len(), capacity = table.cells.capacity());
        Some(table)
    }

    pub(crate) fn publish(&mut self, node: NodeId, table: &mut Table) -> Option<Arc<Table>> {
        let remaining = self.remaining.get(node.0).copied().unwrap_or(0);
        if remaining < 2
            || table.cells.capacity() > LARGE_RESULT_CELL_CAPACITY
            || table.cells.len() != table.count.0.saturating_mul(table.kinds.len())
        {
            tracing::trace!(target: "ryo::diagnostic::file_program", event = "publish_refused",
            node = node.0, remaining, cells = table.cells.len(),
            capacity = table.cells.capacity(), rows = table.count.0,
            columns = table.kinds.len(), reason = if remaining < 2 {
                "consumers"
            } else if table.cells.capacity() > LARGE_RESULT_CELL_CAPACITY {
                "capacity"
            } else {
                "nonrectangular"
            });
            return None;
        }
        let large = table.cells.capacity() > SMALL_RESULT_CELL_CAPACITY;
        if let Some((occupied, _)) =
            self.entries.iter().enumerate().find(|(_, (_, held))| {
                (held.cells.capacity() > SMALL_RESULT_CELL_CAPACITY) == large
            })
        {
            // A later, larger shared cut may save more recomputation than a
            // small earlier cut. Releasing a complete result only causes its
            // future consumers to run the ordinary operator again.
            let candidate = table.cells.len().saturating_mul(remaining - 1);
            let (id, held) = &self.entries[occupied];
            let score = held
                .cells
                .len()
                .saturating_mul(self.remaining[id.0].saturating_sub(1));
            if Arc::strong_count(held) != 1 || candidate <= score {
                tracing::trace!(target: "ryo::diagnostic::file_program", event = "publish_refused",
                    node = node.0, remaining, cells = table.cells.len(),
                    capacity = table.cells.capacity(), rows = table.count.0,
                    columns = table.kinds.len(), reason = "retained_slot",
                    candidate, score, held_node = id.0,
                    held_references = Arc::strong_count(held));
                return None;
            }
            let _removed = self.entries.swap_remove(occupied);
        }
        let kept = Arc::new(std::mem::take(table));
        tracing::trace!(target: "ryo::diagnostic::file_program", event = "publish",
            node = node.0, cells = kept.cells.len(), capacity = kept.cells.capacity(), remaining);
        self.entries.push((node, Arc::clone(&kept)));
        self.peak_retained_cells = self.peak_retained_cells.max(
            self.entries
                .iter()
                .map(|entry| entry.1.cells.capacity())
                .sum(),
        );
        Some(kept)
    }

    /// A file rule has finished rendering and handing over its frontier.
    pub(crate) fn finish_rule(&mut self, rule: usize) {
        let Some(consumers) = self.graph.consumers.get(rule) else {
            return;
        };
        for node in consumers {
            if let Some(remaining) = self.remaining.get_mut(node.0) {
                *remaining = remaining.saturating_sub(1);
                if *remaining == 0 {
                    let before = self.entries.len();
                    self.entries.retain(|entry| entry.0 != *node);
                    if self.entries.len() != before {
                        tracing::trace!(target: "ryo::diagnostic::file_program", event = "release",
                            node = node.0, rule);
                    }
                }
            }
        }
    }
}

use ryo_values::domain::rows::types::Table;
