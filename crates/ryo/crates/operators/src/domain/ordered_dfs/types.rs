//! What an ordered depth-first search reads from its operation.
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The root column, the edge columns each row carries, and the root's partition keys.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DfsShape<'a> {
    /// The root column of a root row.
    pub root: ColumnIndex,
    /// The source, destination and edge columns of an edge row, in that order.
    pub edge_columns: [ColumnIndex; 3],
    /// The partition key columns of a root row.
    pub root_keys: &'a [ColumnIndex],
}
