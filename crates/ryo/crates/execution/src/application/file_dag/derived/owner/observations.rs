//! Immutable policy answers; no backend reference escapes.
use super::*;
use ryo_relations::domain::relations::types::ColumnsView;
use ryo_values::domain::plan_parts::types::ColumnIndex;

impl DerivedOwner {
    pub(in crate::application::file_dag) fn selector_work(
        &self,
    ) -> Result<crate::domain::operator::types::RowsFed, OperatingError> {
        let backend = self.backend.borrow();
        let backend = backend.as_ref().ok_or(OperatingError::NotRun)?;
        Ok(crate::domain::operator::types::RowsFed([
            backend.selection_counts.predicate_rows.0,
            0,
            0,
        ]))
    }

    pub(super) fn has_live_equality(
        &self,
        rows: ColumnsView<'_>,
        columns: &[ColumnIndex],
    ) -> EqualityLiveness {
        let count = rows.columns.first().map_or(0, |column| column.0.len());
        let live = self.state.borrow().entries.values().any(|entry| {
            matches!(&entry._plan.request, ViewRequest::Equality { columns: requested } if requested == columns)
                && entry._plan.schemas.first().is_some_and(|schema| schema == rows.kinds)
                && entry.weak.upgrade().is_some_and(|storage| {
                    matches!(storage.view(), Ok(ryo_derived::domain::contract::types::DerivedView::Equality(index)) if index.rows.len() == count)
                })
        });
        if live {
            EqualityLiveness::Live
        } else {
            EqualityLiveness::Absent
        }
    }
}
