//! Borrowed axis resources exposed by the engine; primitives cannot retain cache handles.
use crate::domain::{
    operator::contract::views::IndexedRows, table_cells::types::TreeRows, table_reach::types::Walk,
};
use ryo_relations::domain::relations::types::Position;
use ryo_values::domain::plan_parts::types::ColumnIndex;
/// The marks of the elements of a tree that a membership admits, by element position.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DenseMarks<'a>(pub &'a [bool]);

pub(crate) enum PreparedAxis<'a> {
    Empty,
    Native {
        tree: TreeRows<'a>,
        among: Option<TreeRows<'a>>,
        marks: DenseMarks<'a>,
    },
    Indexed {
        edges: IndexedRows<'a>,
        to: ColumnIndex,
        domain: ryo_derived::domain::contract::types::MembershipView<'a>,
    },
}
#[derive(Debug, Default)]
pub(crate) struct AxisWorkspace {
    pub(crate) walk: Walk,
    pub(crate) found: Vec<Position>,
}
