//! Required rule-local ownership is independent of optional cross-rule cache admission.
use super::relations::RelationResult;
use super::{service::bytes, types::FileDagExecution};
use crate::domain::file_program::types::{NodeId, NodeInput};
use std::sync::Arc;
impl FileDagExecution {
    pub(crate) fn begin_rule(&mut self, rule: usize) {
        if self.current_rule == Some(rule) {
            return;
        }
        self.clear_required();
        if !self.active.get(rule).copied().unwrap_or(false) {
            return;
        }
        self.current_rule = Some(rule);
        let graph = Arc::clone(&self.graph);
        // Immutable bookkeeping is request-prepared; only these touched slots
        // and dynamic successful cuts change for one physical file/rule.
        for (node, count) in &graph.rule_liveness[rule] {
            self.required_remaining[node.0] = *count;
            self.required_inputs[node.0] = true;
            self.required_nodes.push(*node);
        }
        // Canonical IDs are topological. Cancel cached-cut ancestors before
        // visiting their smaller IDs, so unreachable cached children are not pinned.
        for (node, _) in &graph.rule_liveness[rule] {
            if self.required_remaining[node.0] == 0 {
                continue;
            }
            if let Some(table) = self.ready[node.0].as_ref().map(Arc::clone) {
                self.pin_required(*node, table);
                self.required_executed[node.0] = true;
                self.required_inputs[node.0] = false;
                for input in &graph.nodes[node.0].inputs {
                    if let NodeInput::Node(child) = input {
                        self.consume_required(*child);
                    }
                }
            }
        }
    }
    pub(super) fn pin_required(&mut self, node: NodeId, table: Arc<RelationResult>) {
        if self.required_remaining[node.0] == 0 || self.required[node.0].is_some() {
            return;
        }
        let size = bytes(&table);
        self.required_bytes = self.required_bytes.saturating_add(size);
        if self.ready[node.0].is_some() {
            self.overlap_bytes = self.overlap_bytes.saturating_add(size);
        }
        if self.graph.derived_sources[node.0] {
            self.relations.pin_derived(table.as_ref());
        }
        self.required[node.0] = Some(table);
        self.observe_required();
    }
    /// New required-owner metadata is reported separately from result payloads.
    pub(crate) fn required_metadata_bytes(&self) -> usize {
        self.required.capacity() * size_of::<Option<Arc<RelationResult>>>()
            + self.required_remaining.capacity() * size_of::<usize>()
            + self.required_executed.capacity() * size_of::<bool>()
            + self.required_inputs.capacity() * size_of::<bool>()
            + self.required_nodes.capacity() * size_of::<NodeId>()
    }
    pub(super) fn observe_required(&mut self) {
        self.peak_required_bytes = self.peak_required_bytes.max(self.required_bytes);
        let owned = self
            .required_bytes
            .saturating_add(self.retained_bytes)
            .saturating_sub(self.overlap_bytes);
        self.peak_owned_bytes = self.peak_owned_bytes.max(owned);
    }
    pub(super) fn consume_required(&mut self, node: NodeId) {
        if self.required_remaining[node.0] == 0 {
            return;
        }
        self.required_remaining[node.0] -= 1;
        if self.required_remaining[node.0] != 0 {
            return;
        }
        if let Some(table) = self.required[node.0].take() {
            if self.graph.derived_sources[node.0] {
                self.relations.release_derived(table.as_ref());
            }
            let size = bytes(&table);
            self.required_bytes = self.required_bytes.saturating_sub(size);
            if self.ready[node.0].is_some() {
                self.overlap_bytes = self.overlap_bytes.saturating_sub(size);
            }
            self.required_releases += 1;
        }
        if !self.required_executed[node.0] && self.required_inputs[node.0] {
            let graph = Arc::clone(&self.graph);
            for input in &graph.nodes[node.0].inputs {
                if let NodeInput::Node(child) = input {
                    self.consume_required(*child);
                }
            }
        }
    }
    pub(super) fn clear_required(&mut self) {
        for node in self.required_nodes.drain(..) {
            if let Some(table) = self.required[node.0].take() {
                if self.graph.derived_sources[node.0] {
                    self.relations.release_derived(table.as_ref());
                }
                let size = bytes(&table);
                self.required_bytes = self.required_bytes.saturating_sub(size);
                if self.ready[node.0].is_some() {
                    self.overlap_bytes = self.overlap_bytes.saturating_sub(size);
                }
                self.required_releases += 1;
            }
            self.required_remaining[node.0] = 0;
            self.required_executed[node.0] = false;
            self.required_inputs[node.0] = false;
        }
        self.current_rule = None;
        debug_assert_eq!(self.required_bytes, 0);
    }
}
