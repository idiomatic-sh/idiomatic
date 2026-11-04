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
        let mut needed = vec![false; graph.nodes.len()];
        let mut remaining = vec![0_usize; graph.nodes.len()];
        let mut pending = Vec::new();
        for (rule, roots) in graph.boundaries.iter().enumerate() {
            if !active[rule] {
                continue;
            }
            for (_, root) in roots {
                remaining[root.0] += 1;
                pending.push(*root);
            }
        }
        while let Some(node) = pending.pop() {
            if needed[node.0] {
                continue;
            }
            needed[node.0] = true;
            for input in &graph.nodes[node.0].inputs {
                if let NodeInput::Node(child) = input {
                    remaining[child.0] += 1;
                    pending.push(*child);
                }
            }
        }
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
