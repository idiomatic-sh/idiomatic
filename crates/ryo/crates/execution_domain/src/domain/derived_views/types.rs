//! Closed typed requirements, construction storage and immutable borrowed access.
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The derived view an operation requests, and the columns it is built over.
#[derive(Clone, Debug)]
pub enum ViewRequest {
    /// A hash index of rows by equal keys.
    Equality {
        /// The key columns.
        columns: Vec<ColumnIndex>,
    },
    /// A compact adjacency of edge rows, partitioned by key.
    Adjacency {
        /// The columns of each edge's two endpoints.
        endpoints: [ColumnIndex; 2],
        /// The columns the partitions are keyed on.
        keys: Vec<ColumnIndex>,
    },
    /// The members of one column.
    Membership {
        /// The column whose members are collected.
        column: ColumnIndex,
    },
    /// The rows ordered by a typed key.
    PartitionOrder {
        /// The key columns.
        keys: Vec<ColumnIndex>,
    },
    /// One partition's closure adjacency.
    ClosureAdjacency {
        /// The column edges start at.
        source: ColumnIndex,
        /// The column edges end at.
        destination: ColumnIndex,
    },
    /// The adjacency a cycle search reads.
    CycleAdjacency {
        /// The column edges start at.
        source: ColumnIndex,
        /// The column edges end at.
        destination: ColumnIndex,
    },
    /// The pairing of two ordered inputs by key.
    OrderedKeys {
        /// The left input's key columns.
        left_on: Vec<ColumnIndex>,
        /// The right input's key columns.
        right_on: Vec<ColumnIndex>,
        /// The left input's order column.
        left_order: ColumnIndex,
        /// The right input's order column.
        right_order: ColumnIndex,
    },
    /// The prepared roots and ordered edges of a depth-first search.
    OrderedDfs(DfsResources),
    /// The lookup a static join over native tables reduces to.
    NativeLookup {
        /// The left input's key columns.
        left_on: Vec<ColumnIndex>,
        /// The right input's key columns.
        right_on: Vec<ColumnIndex>,
        /// The columns the join reads from its rows.
        view: Vec<ColumnIndex>,
    },
}
/// The hash of a request's identity, prepared once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IdentityHash(pub u64);

pub mod contract;

/// How many of a prepared use's input places the view reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputsUsed(pub u8);

/// What an ordered depth-first search reads: its roots, edges, edge order and keys.
#[derive(Clone, Debug)]
pub struct DfsResources {
    /// The column of the roots.
    pub root: ColumnIndex,
    /// The column edges start at.
    pub source: ColumnIndex,
    /// The column edges end at.
    pub destination: ColumnIndex,
    /// The column that orders the edges.
    pub order: ColumnIndex,
    /// The column of each edge's identity.
    pub edge: ColumnIndex,
    /// The columns the roots are keyed on.
    pub root_keys: Vec<ColumnIndex>,
    /// The columns the edges are keyed on.
    pub edge_keys: Vec<ColumnIndex>,
}
