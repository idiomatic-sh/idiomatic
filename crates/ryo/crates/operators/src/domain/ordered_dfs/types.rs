//! What an ordered depth-first search reads from its operation.
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The root column, the edge columns each row carries, and the partition keys of each side.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DfsShape<'a> {
    /// The root column of a root row.
    pub(crate) root: ColumnIndex,
    /// The source, destination, order and edge columns of an edge row, in that order.
    pub(crate) edge_columns: [ColumnIndex; 4],
    /// The partition key columns of a root row.
    pub(crate) root_keys: &'a [ColumnIndex],
    /// The partition key columns of an edge row, matched to the root's in order.
    pub(crate) edge_keys: &'a [ColumnIndex],
}

/// What an ordered depth-first visit lowers to: the root and edge columns, and the partition each side keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DfsParts {
    /// The root column of a root row.
    pub(crate) root: ColumnIndex,
    /// The source column of an edge row.
    pub(crate) source: ColumnIndex,
    /// The destination column of an edge row.
    pub(crate) destination: ColumnIndex,
    /// The numeric order column of an edge row: a source's edges are visited in its ascending order.
    pub(crate) order: ColumnIndex,
    /// The edge column of an edge row.
    pub(crate) edge: ColumnIndex,
    /// The partition key columns of a root row.
    pub(crate) root_keys: Vec<ColumnIndex>,
    /// The partition key columns of an edge row.
    pub(crate) edge_keys: Vec<ColumnIndex>,
}



