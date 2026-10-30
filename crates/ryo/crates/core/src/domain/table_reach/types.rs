//! The types of the `table_reach` concept: plain data, public fields and derives, with no
//! constructor and no inherent impl.

use ryo_language::domain::relations::types::Value;

use crate::domain::table_model::types::{KeyIndex, Table};
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
