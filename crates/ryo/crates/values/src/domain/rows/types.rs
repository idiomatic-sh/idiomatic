//! Plain row data; no semantic builder, cache, lease or publication policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowIndex(pub usize);
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowCount(pub usize);
/// Flat row values; ownership/publication policy belongs to the caller.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Table {
    pub kinds: Vec<ryo_relations::domain::relations::types::CellKind>,
    pub cells: Vec<ryo_relations::domain::relations::types::Value>,
    pub count: RowCount,
}
