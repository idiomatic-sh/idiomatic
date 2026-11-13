//! The walk counts the graph cache records.
use super::types::CacheBackend;
use ryo_execution_domain::domain::stored_graph::types::{ExaminedVertices, ExpandedVertices};

impl CacheBackend {
    pub(crate) fn observe_graph_walk(
        &mut self,
        expanded: ExpandedVertices,
        examined: ExaminedVertices,
    ) {
        self.graph_counts.expanded.0 += expanded.0;
        self.graph_counts.examined.0 += examined.0;
    }
}
