//! Plain row data; no semantic builder, cache, lease or publication policy.
/// The position of a row in a table, counted from zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowIndex(pub usize);
/// How many rows a table holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowCount(pub usize);
/// Flat row values; ownership/publication policy belongs to the caller.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Table {
    /// The kind of each column, in column order.
    pub kinds: Vec<ryo_relations::domain::relations::types::CellKind>,
    /// Every cell, row after row, each row as wide as `kinds`.
    pub cells: Vec<ryo_relations::domain::relations::types::Value>,
    /// How many rows `cells` holds.
    pub count: RowCount,
}
