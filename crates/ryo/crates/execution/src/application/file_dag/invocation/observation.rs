//! Measurements read actual engine-owned resources, never shadow scratch.
use super::*;
impl InvocationResources<'_> {
    #[cfg(test)]
    pub(crate) fn observe(&self, workspace: &mut super::super::workspace::InvocationWorkspace) {
        #[cfg(feature = "diagnostic-row-progress")]
        let view = self
            .inputs
            .index
            .as_ref()
            .and_then(|index| index.view(&self.inputs.progress));
        #[cfg(not(feature = "diagnostic-row-progress"))]
        let view = self.inputs.index.as_ref().and_then(IndexResource::view);
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
            observed.index_rows = view.rows.count().0;
            observed.index_representation = Some(
                crate::domain::table_model::service::kind(view.rows),
            );
        }
        let mut rows = self
            .inputs
            .closure
            .iter()
            .map(|part| {
                let ryo_derived::domain::contract::types::DerivedView::ClosureAdjacency(view) =
                    part.view()
                        .expect(super::constants::VALIDATED_CLOSURE_PARTITION)
                else {
                    panic!("closure partition family")
                };
                view.table.count.0
            })
            .sum();
        let mut capacity = self
            .inputs
            .closure
            .iter()
            .map(|part| {
                let ryo_derived::domain::contract::types::DerivedView::ClosureAdjacency(view) =
                    part.view()
                        .expect(super::constants::VALIDATED_CLOSURE_PARTITION)
                else {
                    panic!("closure partition family")
                };
                let capacities = ryo_derived::domain::closure_adjacency::service::capacities(view);
                view.table.cells.capacity() + capacities.key.0 + capacities.sources.0
            })
            .sum();
        if let Some(parts) = &self.inputs.partitioned_closure {
            rows = parts
                .current()
                .expect(super::constants::OBSERVED_ACTIVE_PARTITION)
                .table
                .count
                .0;
            capacity = crate::application::file_dag::closure_partitions::partitions_test::working_capacity(parts);
        }
        if !self.inputs.closure.is_empty() || self.inputs.partitioned_closure.is_some() {
            capacity += workspace.primitive.evaluation.rows.row.capacity()
                + workspace.primitive.evaluation.rows.other.capacity()
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
        if let Some(dfs) =
            Self::dfs_view(self.inputs.dfs.as_ref()).expect(super::constants::VALIDATED_DFS_VIEW)
        {
            observed.dfs_adjacency_len = dfs.adjacency.len();
        }
    }
}
