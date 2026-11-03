//! Immutable engine-prepared graph inputs and invocation-bounded traversal work.

use crate::domain::table_model::types::KeyIndex;
use crate::domain::table_scratch::types::WalkStep;
use ryo_relations::domain::relations::types::{Integer, Value};
#[derive(Debug, Default)]
pub(crate) struct CycleGraph {
    pub(crate) values: Vec<Value>,
    pub(crate) starts: Vec<usize>,
    pub(crate) leaving: Vec<usize>,
}
#[derive(Debug, Default)]
pub(crate) struct CycleMarks {
    pub(crate) order: Vec<usize>,
    pub(crate) low: Vec<usize>,
    pub(crate) open: Vec<bool>,
    pub(crate) found: Vec<bool>,
    pub(crate) stack: Vec<usize>,
    pub(crate) component: Vec<usize>,
    pub(crate) walk: Vec<WalkStep>,
    pub(crate) ordered: Vec<Value>,
}
#[derive(Debug)]
pub(crate) struct ClosurePartition {
    pub(crate) key: Vec<Value>,
    pub(crate) table: Table,
    pub(crate) index: KeyIndex,
    pub(crate) sources: Vec<Value>,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum OwnedOrderedKey {
    Text(String),
    Value(Value),
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct OwnedOrderedRow {
    pub(crate) keys: Vec<OwnedOrderedKey>,
    pub(crate) order: Integer,
    pub(crate) row: RowIndex,
}
#[derive(Debug)]
pub(crate) struct OrderedMatching {
    pub(crate) left: Vec<OwnedOrderedRow>,
    pub(crate) right: Vec<OwnedOrderedRow>,
}

/// Lookup layout is immutable while one invocation walks its independent cursor.
#[derive(Debug)]
pub(crate) struct PreparedDfs {
    pub(crate) roots: Table,
    pub(crate) edges: Table,
    pub(crate) root_index: KeyIndex,
    pub(crate) edge_index: KeyIndex,
    pub(crate) layout: crate::domain::table_reach::types::OrderedGraph,
}
#[derive(Debug, Default)]
pub(crate) struct DfsWorkspace {
    pub(crate) walk: crate::domain::table_reach::types::OrderedWalk,
    pub(crate) expanded: usize,
    pub(crate) examined: usize,
}

use ryo_values::domain::rows::types::{RowIndex, Table};
