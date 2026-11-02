//! Typed request graph construction and bounded file-local successful result reuse.

use super::types::{FileProgram, Node, NodeId, NodeInput, RequestGraph};
use crate::domain::table_model::types::Table;
use ryo_language::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{Input, Operand, QueryLiteral, StepId, Test};
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
    operation: Operation,
    inputs: Vec<NodeInput>,
    relations: Vec<Relation>,
}

/// Experimental execution choice fixed for this process and used by both the
/// request digest and file executor. It is disabled in ordinary production runs.
pub(crate) fn enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("RYO_FILE_PROGRAM_GRAPH").is_ok_and(|value| value == "1"))
}

fn simple(operand: &Operand) -> bool {
    matches!(operand, Operand::Column(_) | Operand::Literal(_))
}

/// This first graph slice admits operators whose successful output is determined
/// entirely by typed inputs and authored constants. Row-dependent functions,
/// parameters, stateful traversal and provider-specific failure paths decline.
fn shareable(operation: &Operation) -> bool {
    match operation {
        Operation::Select { comparisons, .. } => comparisons.iter().all(|comparison| {
            simple(&comparison.left)
                && simple(&comparison.right)
                && matches!(comparison.test, Test::Equal | Test::NotEqual)
        }),
        Operation::Project { .. } | Operation::Union { .. } => true,
        Operation::Join { filter, reach, .. } => filter.is_empty() && reach.is_none(),
        _ => false,
    }
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
    if !shareable(operation) {
        return None;
    }
    let inputs = named(&inputs_of(operation))
        .iter()
        .map(|input| node_input(*input, previous))
        .collect::<Option<Vec<_>>>()?;
    let edges = inputs.iter().map(|(_, edge)| *edge).collect();
    let normalized = match operation {
        Operation::Select {
            comparisons, kept, ..
        } => Operation::Select {
            from: inputs[0].0,
            comparisons: comparisons.clone(),
            kept: kept.clone(),
        },
        Operation::Project {
            columns, distinct, ..
        } => Operation::Project {
            from: inputs[0].0,
            columns: columns.clone(),
            distinct: *distinct,
        },
        Operation::Join {
            left_on,
            right_on,
            output,
            filter,
            view,
            reach,
            ..
        } => Operation::Join {
            left: inputs[0].0,
            right: inputs[1].0,
            left_on: left_on.clone(),
            right_on: right_on.clone(),
            output: output.clone(),
            filter: filter.clone(),
            view: view.clone(),
            reach: reach.clone(),
        },
        Operation::Union { .. } => Operation::Union {
            left: inputs[0].0,
            right: inputs[1].0,
        },
        _ => return None,
    };
    Some((normalized, edges))
}

fn hash_operand(operand: &Operand, hash: &mut impl Hasher) {
    match operand {
        Operand::Column(column) => {
            0_u8.hash(hash);
            column.hash(hash);
        }
        Operand::Literal(QueryLiteral::Number(number)) => {
            1_u8.hash(hash);
            number.hash(hash);
        }
        Operand::Literal(QueryLiteral::Text(text)) => {
            2_u8.hash(hash);
            text.hash(hash);
        }
        Operand::Parameter(_) | Operand::Function(_) => {
            unreachable!("shareable excludes dynamic operands")
        }
    }
}

/// Hash is only an accelerator: collisions are always checked with typed
/// Operation equality and exact ordered child IDs before graph nodes merge.
fn fingerprint(operation: &Operation, inputs: &[NodeInput]) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    inputs.hash(&mut hash);
    match operation {
        Operation::Select {
            comparisons, kept, ..
        } => {
            0_u8.hash(&mut hash);
            kept.hash(&mut hash);
            for comparison in comparisons {
                comparison.test.hash(&mut hash);
                hash_operand(&comparison.left, &mut hash);
                hash_operand(&comparison.right, &mut hash);
            }
        }
        Operation::Project {
            columns, distinct, ..
        } => {
            1_u8.hash(&mut hash);
            columns.hash(&mut hash);
            (*distinct == ryo_values::domain::plan_parts::types::RepeatedRows::Dropped)
                .hash(&mut hash);
        }
        Operation::Join {
            left_on,
            right_on,
            output,
            view,
            ..
        } => {
            2_u8.hash(&mut hash);
            left_on.hash(&mut hash);
            right_on.hash(&mut hash);
            for column in output {
                (column.side == ryo_values::domain::plan_parts::types::Side::Left).hash(&mut hash);
                column.column.hash(&mut hash);
            }
            view.hash(&mut hash);
        }
        Operation::Union { .. } => 3_u8.hash(&mut hash),
        _ => unreachable!("canonical operation is shareable"),
    }
    hash.finish()
}

/// Compile one DAG for the entire ordered request. Unsupported descendants
/// break the chain; no raw Debug serialization or source-origin text is a key.
pub(crate) fn compile(plans: &[Plan]) -> RequestGraph {
    let mut nodes = Vec::<CanonicalNode>::new();
    let mut cuts = Vec::with_capacity(plans.len());
    let mut buckets = HashMap::<u64, Vec<NodeId>>::new();
    for plan in plans {
        let mut mapped = Vec::with_capacity(plan.steps.len());
        let mut rule_cuts = Vec::with_capacity(plan.steps.len());
        for (index, step) in plan.steps.iter().enumerate() {
            let node = if step.scope == StepScope::PerFile {
                canonical(&step.operation, &mapped).map(|(operation, inputs)| {
                    let hash = fingerprint(&operation, &inputs);
                    if let Some(found) = buckets.get(&hash).and_then(|bucket| {
                        bucket.iter().copied().find(|id| {
                            let existing = &nodes[id.0];
                            existing.operation == operation && existing.inputs == inputs
                        })
                    }) {
                        return found;
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
                    let id = NodeId(nodes.len());
                    nodes.push(CanonicalNode {
                        operation,
                        inputs,
                        relations,
                    });
                    buckets.entry(hash).or_default().push(id);
                    id
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
    RequestGraph {
        consumers,
        plans: plans
            .iter()
            .map(|plan| std::ptr::from_ref(plan).addr())
            .collect(),
        nodes: nodes
            .into_iter()
            .map(|node| Node {
                relations: node.relations,
            })
            .collect(),
        cuts,
    }
}

impl FileProgram {
    pub(crate) fn new(
        graph: Arc<RequestGraph>,
        active: impl Fn(usize) -> bool,
        complete: &[Relation],
    ) -> Self {
        tracing::trace!(target: "ryo::diagnostic::file_program", event = "complete_relations",
            complete = ?complete);
        let mut remaining = vec![0_usize; graph.nodes.len()];
        for (rule, consumers) in graph.consumers.iter().enumerate() {
            if !active(rule) {
                continue;
            }
            for node in consumers {
                if !graph.nodes[node.0]
                    .relations
                    .iter()
                    .all(|relation| complete.contains(relation))
                {
                    continue;
                }
                remaining[node.0] += 1;
            }
        }
        Self {
            graph,
            remaining,
            entries: Vec::new(),
            shared_executions: 0,
            peak_retained_cells: 0,
        }
    }

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
