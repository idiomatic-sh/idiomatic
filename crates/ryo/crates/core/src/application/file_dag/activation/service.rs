//! Prepare structural eligibility and edge counts once for an exact file activation.
use super::types::{ActivatedSchedule, ActivationCache, RELATIONS};
use crate::domain::file_program::types::{NodeInput, RequestGraph};
use ryo_relations::domain::relations::types::Relation;
use std::sync::Arc;

impl ActivationCache {
    pub(in crate::application) fn activate(
        &mut self,
        graph: &Arc<RequestGraph>,
        governs: impl Fn(usize) -> bool,
        complete: &[Relation],
    ) -> Arc<ActivatedSchedule> {
        self.governing.clear();
        self.governing.extend((0..graph.cuts.len()).map(governs));
        let mut eligibility = [false; RELATIONS];
        for relation in complete {
            eligibility[ryo_relations::domain::relations::service::number_of(*relation).0] = true;
        }
        if let Some(entry) = self.entry.as_ref().filter(|entry| {
            Arc::ptr_eq(&entry.graph, graph)
                && entry.governing == self.governing
                && entry.complete == eligibility
        }) {
            return Arc::clone(entry);
        }
        self.entry = None;
        let active = self.governing.clone();
        let reusable = graph
            .nodes
            .iter()
            .map(|node| {
                node.schema_valid
                    && node.relations.iter().all(|relation| {
                        eligibility
                            [ryo_relations::domain::relations::service::number_of(*relation).0]
                    })
            })
            .collect::<Vec<_>>();
        let roots = graph
            .boundaries
            .iter()
            .enumerate()
            .filter(|(rule, _)| active[*rule])
            .flat_map(|(_, roots)| roots.iter().map(|(_, root)| *root));
        let remaining = remaining_for(graph, roots, |_| false);
        let entry = Arc::new(ActivatedSchedule {
            graph: Arc::clone(graph),
            governing: self.governing.clone(),
            complete: eligibility,
            active,
            reusable,
            remaining,
        });
        self.entry = Some(Arc::clone(&entry));
        entry
    }
}

/// The same edge-count algorithm serves file roots and supplied project leaves.
/// Root selection is phase orchestration; there is still one node scheduler.
pub(in crate::application::file_dag) fn remaining_for(
    graph: &RequestGraph,
    roots: impl IntoIterator<Item = crate::domain::file_program::types::NodeId>,
    supplied: impl Fn(crate::domain::file_program::types::NodeId) -> bool,
) -> Vec<usize> {
    let mut needed = vec![false; graph.nodes.len()];
    let mut remaining = vec![0_usize; graph.nodes.len()];
    let mut pending = Vec::new();
    for root in roots {
        remaining[root.0] += 1;
        pending.push(root);
    }
    while let Some(node) = pending.pop() {
        if needed[node.0] {
            continue;
        }
        needed[node.0] = true;
        if supplied(node) {
            continue;
        }
        for input in &graph.nodes[node.0].inputs {
            if let NodeInput::Node(child) = input {
                remaining[child.0] += 1;
                pending.push(*child);
            }
        }
    }
    remaining
}
