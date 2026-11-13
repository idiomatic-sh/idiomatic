//! The types of the `dfs` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use crate::domain::equality::types::KeyIndex;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::Table;

/// The place of an edge in the ordering of edges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeSlot(pub usize);

/// The places of the edges that leave one vertex.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeRun {
    /// The first place of the run.
    pub start: EdgeSlot,
    /// The place after the last of the run.
    pub end: EdgeSlot,
}

/// The columns an ordered DFS lookup reads from its roots and its edges.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DfsColumns<'a> {
    /// The root column.
    pub root: ColumnIndex,
    /// The root key columns.
    pub root_keys: &'a [ColumnIndex],
    /// The edge source column.
    pub source: ColumnIndex,
    /// The edge destination column.
    pub destination: ColumnIndex,
    /// The edge order column.
    pub order: ColumnIndex,
    /// The edge identity column.
    pub edge: ColumnIndex,
    /// The edge key columns.
    pub edge_keys: &'a [ColumnIndex],
}

/// Prepared roots and edges with their canonical columns, indexes and per-edge runs.
#[derive(Debug)]
pub struct PreparedDfs {
    /// The roots with their canonical columns appended.
    pub roots: Table,
    /// The edges with their canonical columns appended.
    pub edges: Table,
    /// The index of the roots by key.
    pub root_index: KeyIndex,
    /// The index of the edges by key.
    pub edge_index: KeyIndex,
    /// The root key columns, the root last.
    pub root_columns: Vec<ColumnIndex>,
    /// The edge key columns, source and destination last.
    pub edge_columns: Vec<ColumnIndex>,
    /// For each edge, the run of edges that share its key.
    pub adjacency: Vec<EdgeRun>,
}
