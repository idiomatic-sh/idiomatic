//! Pure borrowed-value and preparation fixtures; no executor or admission alternative.
use ryo_relations::domain::relations::types::{CellKind, Value};
/// Complete computed fixture rows for portable derived semantics controls.
#[cfg(any(test, feature = "fixtures"))]
#[derive(Clone, Copy, Debug)]
pub struct CompleteTableRows<'a>(pub &'a ryo_values::domain::rows::types::Table);
#[cfg(any(test, feature = "fixtures"))]
impl ryo_derived::domain::row_cells::types::RowCells for CompleteTableRows<'_> {
    fn bulk(&self) -> Option<ryo_derived::domain::row_cells::types::BulkRows<'_>> {
        Some(ryo_derived::domain::row_cells::types::BulkRows::Computed(
            self.0,
        ))
    }
    fn validate(&self) -> Result<(), ryo_derived::domain::fault::types::DerivedFault> {
        (self.0.count.0.checked_mul(self.0.kinds.len()) == Some(self.0.cells.len()))
            .then_some(())
            .ok_or(ryo_derived::domain::fault::types::DerivedFault::ShapeMismatch)
    }
    fn kinds(&self) -> &[CellKind] {
        &self.0.kinds
    }
    fn count(&self) -> ryo_values::domain::rows::types::RowCount {
        self.0.count
    }
    fn cell(
        &self,
        row: ryo_values::domain::rows::types::RowIndex,
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    ) -> Option<Value> {
        ryo_values::domain::rows::service::row_in(self.0, row)?
            .get(column.0 as usize)
            .copied()
    }
}
