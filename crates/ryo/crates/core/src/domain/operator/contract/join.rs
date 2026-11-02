//! Native lookup is immutable semantic data, with no cache or service capabilities.
use crate::domain::table_cells::types::TreeRows;
use crate::domain::tree_join::collections::strategies::types::Finding;
#[derive(Clone, Copy)]
pub(crate) struct NativeJoin<'a> {
    pub tree: TreeRows<'a>,
    pub finding: Finding,
}
