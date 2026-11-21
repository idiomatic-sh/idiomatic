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

/// An entity of one walk's graph, as the number the caller knows it by within one partition.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::ordered_dfs::types::WalkEntity;
/// assert!(WalkEntity(1) < WalkEntity(2));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WalkEntity(pub u64);

/// One edge of a walk's graph: where it leaves, where it arrives, and its order among the edges of its source.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::ordered_dfs::types::{WalkEdge, WalkEntity};
/// use ryo_relations::domain::relations::types::Integer;
/// let edge = WalkEdge { source: WalkEntity(1), destination: WalkEntity(2), order: Integer(0) };
/// assert_eq!(edge.source, WalkEntity(1));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WalkEdge {
    /// Where the edge leaves.
    pub source: WalkEntity,
    /// Its order among its source's edges.
    pub order: ryo_relations::domain::relations::types::Integer,
    /// Where it arrives.
    pub destination: WalkEntity,
}

/// The place of an edge among the edges a walk was given.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::ordered_dfs::types::EdgePlace;
/// assert_eq!(EdgePlace(0), EdgePlace::default());
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgePlace(pub usize);
