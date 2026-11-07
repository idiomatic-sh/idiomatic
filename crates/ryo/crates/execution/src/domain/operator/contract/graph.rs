//! Immutable engine-prepared graph inputs and invocation-bounded traversal work.

use ryo_derived::domain::equality::types::KeyIndex;
use ryo_relations::domain::relations::types::{Integer, Value};

/// Lookup layout is immutable while one invocation walks its independent cursor.

#[derive(Debug, Default)]
pub(crate) struct DfsWorkspace {
    pub(crate) walk: crate::domain::table_reach::types::OrderedWalk,
    pub(crate) expanded: usize,
    pub(crate) examined: usize,
}

use ryo_values::domain::rows::types::{RowIndex, Table};
