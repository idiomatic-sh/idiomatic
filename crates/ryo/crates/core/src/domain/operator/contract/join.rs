//! Native lookup is immutable semantic data, with no cache or service capabilities.
use crate::domain::table_cells::types::TreeRows;

use ryo_derived::domain::contract::types::Finding;
#[derive(Clone, Copy)]
pub(crate) struct NativeJoin<'a> {
    pub tree: TreeRows<'a>,
    pub finding: Finding,
}
