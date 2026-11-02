//! Borrowed axis resources exposed by the engine; primitives cannot retain cache handles.
use crate::domain::{
    derived_views::types::MembershipData, operator::contract::views::IndexedRows,
    table_cells::types::TreeRows, table_reach::types::Walk,
};
use ryo_values::domain::plan_parts::types::ColumnIndex;
pub(crate) enum PreparedAxis<'a> {
    Empty,
    Native {
        tree: TreeRows<'a>,
        among: Option<TreeRows<'a>>,
        marks: &'a [bool],
    },
    Indexed {
        edges: IndexedRows<'a>,
        to: ColumnIndex,
        domain: &'a MembershipData,
    },
}
#[derive(Debug, Default)]
pub(crate) struct AxisWorkspace {
    pub(crate) walk: Walk,
    pub(crate) found: Vec<u32>,
}
