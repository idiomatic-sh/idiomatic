//! Immutable engine-prepared graph inputs and invocation-bounded traversal work. The engine's walks over its
//! closure partitions and its ordered depth-first graph, and its prepared adjacency and ordered keys, are
//! its strategies behind the operators' closure, depth-first, cycle and ordered-match driver ports.
use super::driver::LeaseDriver;
use super::views::PreparedAdjacency;
use super::types::PreparedInputs;
use super::types::{ExaminedCount, ExpandedCount};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_reach::types::{Edges, OrderedVisit};
use ryo_operators::domain::closure::types::{ClosureDriver, ClosureEvaluation};
use ryo_operators::domain::contract::errors::ContractError;
use ryo_operators::domain::contract::types::PortableOperatorFault as Fault;
use ryo_operators::domain::contract::types::{RowDriver, RowRead, VertexPlace};
use ryo_operators::domain::closure::types::PartitionPlace;
use ryo_operators::domain::cycles::types::{
    CycleDriver, CyclesEvaluation, EdgePlace, NodeCount, NodeNumber,
};
use ryo_operators::domain::ordered_dfs::types::{DfsDriver, DfsEvaluation, OrderedEdge};
use ryo_operators::domain::ordered_match::types::{OrderedMatchDriver, OrderedMatchEvaluation};
use ryo_relations::domain::relations::types::{Count, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex};

impl CycleDriver for PreparedAdjacency<'_> {
    fn nodes(&self) -> NodeCount {
        NodeCount(self.0.values.len())
    }
    fn value(&self, node: NodeNumber) -> Option<Value> {
        self.0.values.get(node.0).copied()
    }
    fn edge(&self, node: NodeNumber, place: EdgePlace) -> Option<NodeNumber> {
        let starts = &self.0.starts.0;
        let at = starts.get(node.0)?.checked_add(place.0)?;
        (at < *starts.get(node.0 + 1)?)
            .then(|| self.0.leaving.0.get(at).copied().map(NodeNumber))
            .flatten()
    }
}

impl<'f> LeaseDriver<'_, 'f> {
    /// The walked edges of the depth-first graph: its edge keys but the last, toward the last.
    fn dfs_edges(&self) -> Result<Edges<'f>, OperatingError> {
        let graph = self.graph.dfs.ok_or(OperatingError::WrongBinding)?;
        let columns = graph
            .edge_columns
            .get(
                ..graph
                    .edge_columns
                    .len()
                    .checked_sub(1)
                    .ok_or(OperatingError::ShapeMismatch)?,
            )
            .ok_or(OperatingError::ShapeMismatch)?;
        let target = *graph
            .edge_columns
            .last()
            .ok_or(OperatingError::ShapeMismatch)?;
        Ok(Edges {
            table: &graph.edges,
            order: &graph.edge_index,
            from: columns,
            to: target,
        })
    }

    fn reach_partition(
        &mut self,
        partition: PartitionPlace,
        source: Value,
    ) -> Result<Count, OperatingError> {
        let partition = self
            .graph
            .closure
            .get(super::views::PartitionIndex(partition.0))?;
        let walk = self
            .lease
            .workspace
            .closure_walk
            .as_mut()
            .ok_or(OperatingError::WrongBinding)?;
        crate::domain::table_reach::service::reach_all(
            &Edges {
                table: &partition.table,
                order: &partition.index,
                from: &[ColumnIndex(0)],
                to: ColumnIndex(1),
            },
            source,
            walk,
        );
        Ok(Count(walk.reached.len()))
    }

    fn next_visit(&mut self, partition: &[Value]) -> Result<Option<OrderedEdge>, OperatingError> {
        let graph = self.graph.dfs.ok_or(OperatingError::WrongBinding)?;
        let edges = self.dfs_edges()?;
        let work = self
            .lease
            .workspace
            .dfs
            .as_mut()
            .ok_or(OperatingError::WrongBinding)?;
        let visit = crate::domain::table_reach::service::next_ordered(
            &edges,
            graph,
            partition,
            &mut work.walk,
        )?;
        if visit.is_none() {
            // The walk from this root is over: what it expanded and examined counts for the invocation.
            work.expanded.0 += work.walk.expanded.0;
            work.examined.0 += work.walk.examined.0;
        }
        Ok(visit.map(|OrderedVisit { edge, ordinal }| OrderedEdge { edge, ordinal }))
    }
}

impl ClosureDriver for LeaseDriver<'_, '_> {
    fn partitions(&mut self) -> Result<Count, Fault> {
        if self.interrupted() {
            return Err(Fault::Interrupted);
        }
        let closure = self.resources.closure();
        let closure = self.hold(closure)?;
        self.graph.closure = closure;
        Ok(Count(closure.len().0))
    }
    fn partition_key(
        &mut self,
        partition: PartitionPlace,
        key: &mut Vec<Value>,
    ) -> Result<(), Fault> {
        let found = self
            .graph
            .closure
            .get(super::views::PartitionIndex(partition.0))
            .map(|partition| {
                key.clear();
                key.extend_from_slice(&partition.key);
            });
        self.hold(found)
    }
    fn sources(&self, partition: PartitionPlace) -> Count {
        Count(
            self.graph
                .closure
                .get(super::views::PartitionIndex(partition.0))
                .map_or(0, |partition| partition.sources.len()),
        )
    }
    fn source(&self, partition: PartitionPlace, at: VertexPlace) -> Option<Value> {
        self.graph
            .closure
            .get(super::views::PartitionIndex(partition.0))
            .ok()?
            .sources
            .get(at.0)
            .copied()
    }
    fn reach(&mut self, partition: PartitionPlace, source: Value) -> Result<Count, Fault> {
        if self.interrupted() {
            return Err(Fault::Interrupted);
        }
        let reached = self.reach_partition(partition, source);
        self.hold(reached)
    }
    fn reached(&self, at: VertexPlace) -> Option<Value> {
        self.lease
            .workspace
            .closure_walk
            .as_ref()?
            .reached
            .get(at.0)
            .copied()
    }
}

impl DfsDriver for LeaseDriver<'_, '_> {
    fn roots(&mut self) -> Result<RowCount, Fault> {
        if self.interrupted() {
            return Err(Fault::Interrupted);
        }
        let graph = self
            .resources
            .dfs()
            .and_then(|graph| graph.ok_or(OperatingError::WrongBinding));
        let graph = self.hold(graph)?;
        self.graph.dfs = Some(graph);
        let checked = self.dfs_edges().map(|_| ()).and_then(|()| {
            let work = self
                .lease
                .workspace
                .dfs
                .as_mut()
                .ok_or(OperatingError::WrongBinding)?;
            work.expanded = ExpandedCount::default();
            work.examined = ExaminedCount::default();
            Ok(())
        });
        self.hold(checked)?;
        Ok(RowCount(graph.root_index.sorted.len()))
    }
    fn root(&self, at: RowIndex, row: &mut Vec<Value>) -> RowRead {
        let Some(graph) = self.graph.dfs else {
            return RowRead::Missing;
        };
        let Some(root) =
            graph.root_index.sorted.get(at.0).and_then(|sorted| {
                ryo_values::domain::rows::service::row_in(&graph.roots, *sorted)
            })
        else {
            return RowRead::Missing;
        };
        row.clear();
        row.extend_from_slice(root);
        RowRead::Read
    }
    fn root_columns(&self, columns: &mut Vec<ColumnIndex>) {
        columns.clear();
        if let Some(graph) = self.graph.dfs {
            columns.extend_from_slice(&graph.root_columns);
        }
    }
    fn begin(&mut self, partition: &[Value], root: Value) -> Result<(), Fault> {
        if self.interrupted() {
            return Err(Fault::Interrupted);
        }
        let begun = self
            .graph
            .dfs
            .ok_or(OperatingError::WrongBinding)
            .and_then(|graph| {
                let edges = self.dfs_edges()?;
                let work = self
                    .lease
                    .workspace
                    .dfs
                    .as_mut()
                    .ok_or(OperatingError::WrongBinding)?;
                crate::domain::table_reach::service::begin_ordered(
                    &edges,
                    graph,
                    partition,
                    root,
                    &mut work.walk,
                );
                Ok(())
            });
        self.hold(begun)
    }
    fn next(&mut self, partition: &[Value]) -> Result<Option<OrderedEdge>, Fault> {
        if self.interrupted() {
            return Err(Fault::Interrupted);
        }
        let visit = self.next_visit(partition);
        self.hold(visit)
    }
    fn edge(&self, at: RowIndex, row: &mut Vec<Value>) -> RowRead {
        let Some(edge) = self
            .graph
            .dfs
            .and_then(|graph| ryo_values::domain::rows::service::row_in(&graph.edges, at))
        else {
            return RowRead::Missing;
        };
        row.clear();
        row.extend_from_slice(edge);
        RowRead::Read
    }
}

impl ClosureEvaluation for LeaseDriver<'_, '_> {
    fn closure(&mut self) -> &mut dyn ClosureDriver {
        self
    }
}

impl DfsEvaluation for LeaseDriver<'_, '_> {
    fn dfs(&mut self) -> &mut dyn DfsDriver {
        self
    }
}

impl CyclesEvaluation for LeaseDriver<'_, '_> {
    fn cycles(&mut self) -> Result<&dyn CycleDriver, Fault> {
        let graph = self
            .resources
            .cycles()
            .and_then(|graph| graph.ok_or(OperatingError::WrongBinding));
        let graph = self.hold(graph)?;
        self.graph.cycles = Some(super::views::PreparedAdjacency(graph));
        self.graph
            .cycles
            .as_ref()
            .map(|graph| graph as &dyn CycleDriver)
            .ok_or(Fault::WrongBinding)
    }
}

impl OrderedMatchEvaluation for LeaseDriver<'_, '_> {
    fn ordered_match(&mut self) -> Result<&mut dyn OrderedMatchDriver, Fault> {
        let checked = match self.inputs {
            PreparedInputs::Views(inputs) => inputs
                .first()
                .ok_or(OperatingError::MissingInput { position: 1 })
                .and_then(|_| {
                    inputs
                        .get(1)
                        .ok_or(OperatingError::MissingInput { position: 2 })
                })
                .and_then(|_| self.resources.ordered())
                .and_then(|ordered| ordered.ok_or(OperatingError::WrongBinding)),
            PreparedInputs::Row { .. } | PreparedInputs::RequestSource(_) => {
                Err(OperatingError::WrongOperation)
            }
        };
        self.graph.ordered = Some(self.hold(checked)?);
        Ok(self)
    }
}

impl OrderedMatchDriver for LeaseDriver<'_, '_> {
    fn left_len(&self) -> RowCount {
        RowCount(self.graph.ordered.map_or(0, |ordered| ordered.left.len()))
    }
    fn right_len(&self) -> RowCount {
        RowCount(self.graph.ordered.map_or(0, |ordered| ordered.right.len()))
    }
    fn compare_keys(&self, left: RowIndex, right: RowIndex) -> std::cmp::Ordering {
        self.graph
            .ordered
            .map_or(std::cmp::Ordering::Equal, |ordered| {
                ordered.left[left.0].keys.cmp(&ordered.right[right.0].keys)
            })
    }
    fn right_order_before_or_equal(
        &self,
        left: RowIndex,
        right: RowIndex,
    ) -> ryo_operators::domain::ordered_match::types::KeyOrder {
        use ryo_operators::domain::ordered_match::types::KeyOrder;
        match self.graph.ordered {
            Some(ordered) if ordered.right[right.0].order <= ordered.left[left.0].order => {
                KeyOrder::BeforeOrEqual
            }
            Some(_) | None => KeyOrder::After,
        }
    }
    fn visit_pair(
        &mut self,
        left: RowIndex,
        right: RowIndex,
        project: &mut dyn FnMut(&[Value], &[Value]) -> Result<(), Fault>,
    ) -> Result<(), Fault> {
        if self.interrupted() {
            return Err(Fault::Interrupted);
        }
        let (Some(ordered), PreparedInputs::Views(inputs)) = (self.graph.ordered, self.inputs)
        else {
            return self.hold(Err(OperatingError::WrongBinding));
        };
        let (Some(left_rows), Some(right_rows)) = (inputs.first(), inputs.get(1)) else {
            return self.hold(Err(OperatingError::ShapeMismatch));
        };
        let (left_buffer, right_buffer) = (&mut self.left_buffer, &mut self.right_buffer);
        let pair = left_rows
            .read(ordered.left[left.0].row, left_buffer)
            .ok_or(OperatingError::ShapeMismatch)
            .and_then(|left| {
                right_rows
                    .read(ordered.right[right.0].row, right_buffer)
                    .ok_or(OperatingError::ShapeMismatch)
                    .map(|right| project(left, right))
            });
        self.hold(pair)?
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), ContractError> {
        RowDriver::emit(self, row).map_err(|_| ContractError::Interrupted)
    }
}
