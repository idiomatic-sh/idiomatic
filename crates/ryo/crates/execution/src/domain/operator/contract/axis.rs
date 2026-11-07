//! Borrowed axis resources exposed by the engine; primitives cannot retain cache handles.
use crate::domain::{
    operator::contract::views::IndexedRows, table_cells::types::TreeRows, table_reach::types::Walk,
};
use ryo_relations::domain::relations::types::Position;
use ryo_values::domain::plan_parts::types::ColumnIndex;
pub(crate) enum PreparedAxis<'a> {
    Empty,
    Native {
        tree: TreeRows<'a>,
        among: Option<TreeRows<'a>>,
        marks: ryo_derived::domain::positions::types::DenseMarks<'a>,
    },
    Indexed {
        edges: IndexedRows<'a>,
        to: ColumnIndex,
        domain: ryo_derived::domain::membership::types::MembershipView<'a>,
    },
}
#[derive(Debug, Default)]
pub(crate) struct AxisWorkspace {
    pub(crate) walk: Walk,
    pub(crate) found: Vec<Position>,
}
