//! The `GraphWalks` value: the closure partitions and the ordered depth-first graph one evaluation acquired, the
//! engine's walks over them behind the operators' closure and depth-first ports.

use super::super::row_delivery::value::RowDelivery;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::failure::types::LatchState;
use crate::domain::operator::contract::types::{ExaminedCount, ExpandedCount};
use crate::domain::operator::contract::views::types::{ClosureViews, PartitionIndex};
use crate::domain::table_reach::types::{Edges, OrderedVisit};
use ryo_derived::domain::closure_adjacency::types::ClosurePartition;
use ryo_derived::domain::dfs::types::PreparedDfs;
use ryo_operators::domain::closure::types::PartitionPlace;
use ryo_operators::domain::contract::types::PortableOperatorFault as Fault;
use ryo_operators::domain::contract::types::{RowRead, VertexPlace};
use ryo_operators::domain::ordered_dfs::types::OrderedEdge;
use ryo_relations::domain::relations::types::{Count, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex};

/// The closure partitions and depth-first graph an evaluation acquired.
#[derive(Default)]
pub(crate) struct GraphWalks<'f> {
    closure: ClosureViews<'f>,
    dfs: Option<&'f PreparedDfs>,
}

impl<'f> GraphWalks<'f> {
    fn partition(&self, partition: PartitionPlace) -> Result<&'f ClosurePartition, OperatingError> {
        crate::domain::operator::contract::views::service::get(
            self.closure,
            PartitionIndex(partition.0),
        )
    }

    /// The walked edges of the depth-first graph: its edge keys but the last, toward the last.
    fn dfs_edges(&self) -> Result<Edges<'f>, OperatingError> {
        let graph = self.dfs.ok_or(OperatingError::WrongBinding)?;
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
        &self,
        rows: &mut RowDelivery<'_, '_>,
        partition: PartitionPlace,
        source: Value,
    ) -> Result<Count, OperatingError> {
        let partition = self.partition(partition)?;
        let walk = rows
            .workspace()
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

    fn next_visit(
        &self,
        rows: &mut RowDelivery<'_, '_>,
        partition: &[Value],
    ) -> Result<Option<OrderedEdge>, OperatingError> {
        let graph = self.dfs.ok_or(OperatingError::WrongBinding)?;
        let edges = self.dfs_edges()?;
        let work = rows
            .workspace()
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

    /// Acquires the closure partitions and answers how many there are.
    pub(crate) fn partitions(&mut self, rows: &mut RowDelivery<'_, 'f>) -> Result<Count, Fault> {
        if rows.interrupted() == LatchState::Failed {
            return Err(Fault::Interrupted);
        }
        let closure = rows.resources().closure();
        self.closure = rows.hold(closure)?;
        Ok(Count(
            crate::domain::operator::contract::views::service::len(self.closure).0,
        ))
    }

    /// One partition's key, into `key`.
    pub(crate) fn partition_key(
        &self,
        rows: &mut RowDelivery<'_, '_>,
        partition: PartitionPlace,
        key: &mut Vec<Value>,
    ) -> Result<(), Fault> {
        let found = self.partition(partition).map(|partition| {
            key.clear();
            key.extend_from_slice(&partition.key);
        });
        rows.hold(found)
    }

    /// How many sources a partition has.
    pub(crate) fn sources(&self, partition: PartitionPlace) -> Count {
        Count(
            self.partition(partition)
                .map_or(0, |partition| partition.sources.len()),
        )
    }

    /// One source of a partition.
    pub(crate) fn source(
        &self,
        rows: &mut RowDelivery<'_, '_>,
        partition: PartitionPlace,
        at: VertexPlace,
    ) -> Result<Option<Value>, Fault> {
        let found = self
            .partition(partition)
            .map(|partition| partition.sources.get(at.0).copied());
        rows.hold(found)
    }

    /// Walks everything a source reaches in a partition, answering how many.
    pub(crate) fn reach(
        &self,
        rows: &mut RowDelivery<'_, '_>,
        partition: PartitionPlace,
        source: Value,
    ) -> Result<Count, Fault> {
        if rows.interrupted() == LatchState::Failed {
            return Err(Fault::Interrupted);
        }
        let reached = self.reach_partition(rows, partition, source);
        rows.hold(reached)
    }

    /// One vertex the last reach found.
    pub(crate) fn reached(rows: &RowDelivery<'_, '_>, at: VertexPlace) -> Option<Value> {
        rows.workspace_view()
            .closure_walk
            .as_ref()?
            .reached
            .get(at.0)
            .copied()
    }

    /// Acquires the depth-first graph, resets its counts and answers how many roots it has.
    pub(crate) fn roots(&mut self, rows: &mut RowDelivery<'_, 'f>) -> Result<RowCount, Fault> {
        if rows.interrupted() == LatchState::Failed {
            return Err(Fault::Interrupted);
        }
        let graph = rows
            .resources()
            .dfs()
            .and_then(|graph| graph.ok_or(OperatingError::WrongBinding));
        let graph = rows.hold(graph)?;
        self.dfs = Some(graph);
        let checked = self.dfs_edges().map(|_| ()).and_then(|()| {
            let work = rows
                .workspace()
                .dfs
                .as_mut()
                .ok_or(OperatingError::WrongBinding)?;
            work.expanded = ExpandedCount::default();
            work.examined = ExaminedCount::default();
            Ok(())
        });
        rows.hold(checked)?;
        Ok(RowCount(graph.root_index.sorted.len()))
    }

    /// One root row, in root order, into `row`.
    pub(crate) fn root(&self, at: RowIndex, row: &mut Vec<Value>) -> RowRead {
        let Some(graph) = self.dfs else {
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

    /// The root rows' key columns, into `columns`.
    pub(crate) fn root_columns(&self, columns: &mut Vec<ColumnIndex>) {
        columns.clear();
        if let Some(graph) = self.dfs {
            columns.extend_from_slice(&graph.root_columns);
        }
    }

    /// Begins the ordered walk from a root.
    pub(crate) fn begin(
        &self,
        rows: &mut RowDelivery<'_, '_>,
        partition: &[Value],
        root: Value,
    ) -> Result<(), Fault> {
        if rows.interrupted() == LatchState::Failed {
            return Err(Fault::Interrupted);
        }
        let begun = self
            .dfs
            .ok_or(OperatingError::WrongBinding)
            .and_then(|graph| {
                let edges = self.dfs_edges()?;
                let work = rows
                    .workspace()
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
        rows.hold(begun)
    }

    /// The next edge of the ordered walk.
    pub(crate) fn next(
        &self,
        rows: &mut RowDelivery<'_, '_>,
        partition: &[Value],
    ) -> Result<Option<OrderedEdge>, Fault> {
        if rows.interrupted() == LatchState::Failed {
            return Err(Fault::Interrupted);
        }
        let visit = self.next_visit(rows, partition);
        rows.hold(visit)
    }

    /// One edge row, into `row`.
    pub(crate) fn edge(&self, at: RowIndex, row: &mut Vec<Value>) -> RowRead {
        let Some(edge) = self
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
