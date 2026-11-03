//! Immutable engine-prepared graph inputs and invocation-bounded traversal work.

use crate::domain::table_scratch::types::WalkStep;
use ryo_derived::domain::contract::types::KeyIndex;
use ryo_relations::domain::relations::types::{Integer, Value};

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

/// Lookup layout is immutable while one invocation walks its independent cursor.

#[derive(Debug, Default)]
pub(crate) struct DfsWorkspace {
    pub(crate) walk: crate::domain::table_reach::types::OrderedWalk,
    pub(crate) expanded: usize,
    pub(crate) examined: usize,
}

use ryo_values::domain::rows::types::{RowIndex, Table};
