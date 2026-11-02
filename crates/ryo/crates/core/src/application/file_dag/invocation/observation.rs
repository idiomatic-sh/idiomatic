//! Measurements read actual engine-owned resources, never shadow scratch.
use super::*;
impl InvocationResources<'_> {
    #[cfg(test)]
    pub(crate) fn observe(&self, workspace: &mut super::super::workspace::InvocationWorkspace) {
        let view = self.index.as_ref().map(IndexResource::view);
        let observed = &mut workspace.observation;
        observed.owners_initialized = usize::from(self.index.is_some())
            + usize::from(self.groups.is_some())
            + usize::from(!self.closure.is_empty() || self.partitioned_closure.is_some())
            + usize::from(self.cycles.is_some())
            + usize::from(self.dfs.is_some())
            + usize::from(self.axis.is_some())
            + usize::from(self.cached_graph.is_some())
            + usize::from(workspace.distinct.cells.capacity() > 0);
        if let Some(view) = view {
            observed.index_rows = view.rows.count();
        }
        let mut rows = self.closure.iter().map(|part| part.table.count.0).sum();
        let mut capacity = self
            .closure
            .iter()
            .map(|part| part.table.cells.capacity() + part.key.capacity() + part.sources.capacity())
            .sum();
        if let Some(parts) = &self.partitioned_closure {
            rows = parts.current().table.count.0;
            capacity = parts.working_capacity();
        }
        if !self.closure.is_empty() || self.partitioned_closure.is_some() {
            capacity += workspace.row.capacity()
                + workspace.other.capacity()
                + workspace.closure.key.capacity()
                + workspace.closure.frontier.capacity()
                + workspace.closure.next.capacity()
                + workspace.closure.reached.capacity()
                + workspace.closure.visited.capacity();
            observed.closure_partition_rows = observed.closure_partition_rows.max(rows);
            observed.closure_partition_capacity = observed.closure_partition_capacity.max(capacity);
        }
        if let Some(dfs) = Self::dfs_view(self.dfs.as_ref()).expect("validated DFS view") {
            observed.dfs_adjacency_capacity = dfs.layout.adjacency.capacity();
            observed.dfs_adjacency_len = dfs.layout.adjacency.len();
        }
    }
}
