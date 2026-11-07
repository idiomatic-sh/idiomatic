//! Native lookup is immutable semantic data, with no cache or service capabilities.
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_cells::types::TreeRows;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;

use ryo_derived::domain::native::types::NativeFinding;
#[derive(Clone, Copy)]
pub(crate) struct NativeJoin<'a> {
    pub tree: TreeRows<'a>,
    pub finding: NativeFinding,
}

/// One logical column of a closure row: a column of the partition's key, the source or the vertex it reaches.
pub(crate) fn closure_cell(
    key: &[Value],
    column: ColumnIndex,
    source: &Value,
    reached: &Value,
) -> Result<Value, OperatingError> {
    let at = column.0 as usize;
    if let Some(each) = key.get(at) {
        Ok(*each)
    } else if at == key.len() {
        Ok(*source)
    } else if at == key.len() + 1 {
        Ok(*reached)
    } else {
        Err(OperatingError::ShapeMismatch)
    }
}
