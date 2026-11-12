//! The types of the `axis_resources` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_execution_domain::domain::table_cells::types::TreeRows;
use ryo_values::domain::plan_parts::types::ColumnIndex;

#[derive(Clone, Copy)]
pub(crate) enum AxisMode<'f> {
    Empty,
    Native {
        tree: TreeRows<'f>,
        among: Option<TreeRows<'f>>,
    },
    Indexed {
        from: [ColumnIndex; 1],
        to: ColumnIndex,
    },
}
