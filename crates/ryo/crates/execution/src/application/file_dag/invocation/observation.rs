//! Measurements read actual engine-owned resources, never shadow scratch.
use super::*;
impl InvocationResources<'_> {
    #[cfg(test)]
    pub(crate) fn observe(&self, workspace: &mut super::super::workspace::InvocationWorkspace) {
        let view = self.inputs.index.as_ref().map(IndexResource::view);
        let observed = &mut workspace.observation;
        observed.owners_initialized = usize::from(self.inputs.index.is_some())
            + usize::from(self.inputs.groups.is_some())
            + usize::from(
                !self.inputs.closure.is_empty() || self.inputs.partitioned_closure.is_some(),
            )
            + usize::from(self.inputs.cycles.is_some())
            + usize::from(self.inputs.dfs.is_some())
            + usize::from(self.inputs.axis.is_some())
            + usize::from(self.cached_graph.is_some())
            + usize::from(
                workspace
                    .primitive
                    .distinct
                    .as_ref()
                    .is_some_and(|state| state.rows.cells.capacity() > 0),
            );
        if let Some(view) = view {
            observed.index_rows = view.rows.count();
        }
        let mut rows = self
            .inputs
            .closure
            .iter()
            .map(|part| {
                let ryo_derived::domain::contract::types::DerivedView::ClosureAdjacency(view) =
                    part.view().expect("validated closure partition")
                else {
                    unreachable!("closure partition family")
                };
                view.table().count.0
            })
            .sum();
        let mut capacity = self
            .inputs
            .closure
            .iter()
            .map(|part| {
                let ryo_derived::domain::contract::types::DerivedView::ClosureAdjacency(view) =
                    part.view().expect("validated closure partition")
                else {
                    unreachable!("closure partition family")
                };
                let (keys, sources) = view.capacities();
                view.table().cells.capacity() + keys + sources
            })
            .sum();
        if let Some(parts) = &self.inputs.partitioned_closure {
            rows = parts
                .current()
                .expect("observed active partition")
                .table()
                .count
                .0;
            capacity = parts.working_capacity();
        }
        if !self.inputs.closure.is_empty() || self.inputs.partitioned_closure.is_some() {
            capacity += workspace.primitive.row.capacity()
                + workspace.primitive.other.capacity()
                + workspace
                    .primitive
                    .closure_walk
                    .as_ref()
                    .map_or(0, |walk| walk.key.capacity())
                + workspace
                    .primitive
                    .closure_walk
                    .as_ref()
                    .map_or(0, |walk| walk.frontier.capacity())
                + workspace
                    .primitive
                    .closure_walk
                    .as_ref()
                    .map_or(0, |walk| walk.next.capacity())
                + workspace
                    .primitive
                    .closure_walk
                    .as_ref()
                    .map_or(0, |walk| walk.reached.capacity())
                + workspace
                    .primitive
                    .closure_walk
                    .as_ref()
                    .map_or(0, |walk| walk.visited.capacity());
            observed.closure_partition_rows = observed.closure_partition_rows.max(rows);
            observed.closure_partition_capacity = observed.closure_partition_capacity.max(capacity);
        }
        if let Some(dfs) = Self::dfs_view(self.inputs.dfs.as_ref()).expect("validated DFS view") {
            observed.dfs_adjacency_len = dfs.adjacency().len();
        }
    }
}
