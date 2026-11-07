//! Pure borrowed-value and preparation fixtures; no executor or admission alternative.
use ryo_relations::domain::relations::types::{CellKind, Value};
/// Composition of the actual portable derived use case for current engine tests.
pub(crate) fn program_derived_port()
-> std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort> {
    std::sync::Arc::new(
        crate::outbound::derived::fixtures::fake::DerivedFake::default(),
    )
}
/// Complete computed fixture rows for portable derived semantics controls.
pub(crate) struct CompleteTableRows<'a>(pub &'a ryo_values::domain::rows::types::Table);
impl ryo_derived::domain::contract::types::RowCells for CompleteTableRows<'_> {
    fn bulk(&self) -> Option<ryo_derived::domain::contract::types::BulkRows<'_>> {
        Some(ryo_derived::domain::contract::types::BulkRows::Computed(
            self.0,
        ))
    }
    fn validate(&self) -> Result<(), ryo_derived::domain::contract::types::DerivedFault> {
        (self.0.count.0.checked_mul(self.0.kinds.len()) == Some(self.0.cells.len()))
            .then_some(())
            .ok_or(ryo_derived::domain::contract::types::DerivedFault::ShapeMismatch)
    }
    fn kinds(&self) -> &[CellKind] {
        &self.0.kinds
    }
    fn count(&self) -> usize {
        self.0.count.0
    }
    fn cell(
        &self,
        row: usize,
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    ) -> Option<Value> {
        ryo_values::domain::rows::service::row_in(
            self.0,
            ryo_values::domain::rows::types::RowIndex(row),
        )?
        .get(column.0 as usize)
        .copied()
    }
}
