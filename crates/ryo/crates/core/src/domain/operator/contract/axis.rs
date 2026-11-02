//! Engine-prepared immutable axis resources and bounded invocation traversal work.
use crate::domain::table_cells::types::TreeRows;
use crate::domain::table_model::types::{KeyIndex, Table};
use crate::domain::table_reach::types::Walk;
use ryo_language::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
#[derive(Debug)]
pub(crate) enum PreparedAxis<'a> {
    Empty,
    Native {
        tree: TreeRows<'a>,
        among: Option<TreeRows<'a>>,
        marks: Vec<bool>,
    },
    Table {
        edges: Table,
        index: KeyIndex,
        from: [ColumnIndex; 1],
        to: ColumnIndex,
        domain: hashbrown::HashSet<Value>,
    },
}
#[derive(Debug, Default)]
pub(crate) struct AxisWorkspace {
    pub(crate) walk: Walk,
    pub(crate) found: Vec<u32>,
}
