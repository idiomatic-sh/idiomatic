//! The types of the `table_reach` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl.

use ryo_relations::domain::relations::types::Value;

use ryo_derived::domain::contract::types::KeyIndex;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The room a walk along edges works in, kept from walk to walk.
#[derive(Debug, Default)]
pub(crate) struct Walk {
    /// The key a lookup of the edges is made with.
    pub key: Vec<Value>,
    /// The entities to visit.
    pub frontier: Vec<Value>,
    /// The entities to visit after those.
    pub next: Vec<Value>,
    /// The entities reached, in the order the walk reached them.
    pub reached: Vec<Value>,
    /// The entities already visited.
    pub visited: hashbrown::HashSet<Value>,
}

/// Edges indexed by where they start, to follow from an entity to the ones it leads to.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Edges<'a> {
    /// The edges, one row each.
    pub table: &'a Table,
    /// The rows in the order of where they start.
    pub order: &'a KeyIndex,
    /// The column an edge starts at.
    pub from: &'a [ColumnIndex],
    /// The column an edge ends at.
    pub to: ColumnIndex,
}

/// One reusable iterative DFS, reset before each distinct partition/root.
#[derive(Debug, Default)]
pub(crate) struct OrderedWalk {
    /// Exact partition and current entity lookup key.
    pub key: Vec<Value>,
    /// Next adjacency cursor and end of each active stack frame.
    pub frames: Vec<std::ops::Range<usize>>,
    /// Vertices already expanded, including the root before its first edge.
    pub visited: hashbrown::HashSet<Value>,
    /// Destination to consider only after the previous visit was emitted.
    pub pending: Option<Value>,
    /// Next representable visit ordinal; none means the next visit would overflow.
    pub ordinal: Option<i64>,
    /// Vertices expanded in this root walk.
    pub expanded: usize,
    /// Edges examined in this root walk.
    pub examined: usize,
}

use ryo_values::domain::rows::types::Table;
