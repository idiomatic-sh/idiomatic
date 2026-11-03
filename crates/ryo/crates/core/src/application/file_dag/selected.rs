//! Generation-local selected-row index and direct projected-cell emission.
//! Only the application retains selection metadata; primitives receive borrowed typed rows.

use super::derived::view::AcquiredView;
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, OperatorContext},
};
use crate::domain::operator::contract::views::{BorrowedRows, IndexedRows};
use crate::domain::table_model::{
    service::{joined_selected, key_of, matching_selected},
    types::{KeyIndex, On, SelectedRows},
};
use ryo_language::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn};

/// One complete selected population and an index over only its declared key cells.
/// The source remains borrowed from this physical file; metadata is an Arc clone.
pub(crate) struct SelectedInput<'f> {
    original: SelectedRows<'f>,
    plan: std::sync::Arc<crate::domain::derived_views::types::DerivedContract>,
    index: AcquiredView,
}
impl<'f> SelectedInput<'f> {
    #[cfg(test)]
    pub(crate) fn prepare(
        original: &SelectedRows<'f>,
        columns: &[ColumnIndex],
    ) -> Result<Self, OperatingError> {
        use crate::domain::derived_views::{
            collections::views::registration::registered, types::*,
        };
        let rows = BorrowedRows::Selected {
            source: original.source,
            positions: &original.selection.rows,
            columns: &original.selection.columns,
            kinds: &original.selection.kinds,
        };
        rows.validate()?;
        let plan = std::sync::Arc::new(registered(DerivedKind::Equality).prepare(
            &ViewRequest::Equality {
                columns: columns.to_vec(),
            },
            &[crate::domain::operation_schema::types::InputSchema {
                kinds: rows.kinds(),
                tree: None,
            }],
        )?);
        let mut storage = DerivedStorage::Unbuilt;
        super::derived::owner::build_scoped(
            crate::application::file_dag::route_counts::BuildRoute::Fixture,
            &plan,
            DerivedInputs::Rows(&[rows]),
            &mut storage,
            &mut DerivedScratch::default(),
        )?;
        Self::from_derived(original, plan, std::sync::Arc::new(storage))
    }
    pub(super) fn from_derived(
        original: &SelectedRows<'f>,
        plan: std::sync::Arc<crate::domain::derived_views::types::DerivedContract>,
        storage: std::sync::Arc<crate::domain::derived_views::types::DerivedStorage>,
    ) -> Result<Self, OperatingError> {
        use crate::domain::derived_views::types::DerivedView;
        let index = AcquiredView::new(&plan, storage);
        if !matches!(index.view()?, DerivedView::Equality(_)) {
            return Err(OperatingError::WrongOperation);
        }
        Ok(Self {
            original: original.clone(),
            plan,
            index,
        })
    }
    fn columns(&self) -> &[ColumnIndex] {
        let crate::domain::derived_views::types::ViewRequest::Equality { columns } =
            &self.plan.request
        else {
            unreachable!("prepared equality")
        };
        columns
    }
    fn index(&self) -> &KeyIndex {
        let crate::domain::derived_views::types::DerivedView::Equality(index) =
            self.index.view().expect("validated equality view")
        else {
            unreachable!("validated equality view")
        };
        index
    }
    pub(crate) fn view(&self) -> IndexedRows<'_> {
        IndexedRows {
            rows: BorrowedRows::Selected {
                source: self.original.source,
                positions: &self.original.selection.rows,
                columns: &self.original.selection.columns,
                kinds: &self.original.selection.kinds,
            },
            columns: self.columns(),
            index: self.index(),
        }
    }

    /// Join one left row with matching compact positions in their original bag order.
    /// Only output cells are decoded; no full RHS row or table is materialized.
    pub(crate) fn emit_join(
        &self,
        left_on: &[ColumnIndex],
        output: &[JoinColumn],
        left: &[Value],
        key: &mut Vec<Value>,
        answer: &mut Vec<Value>,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        key_of(left, On::Columns(left_on), key)?;
        for row in matching_selected(&self.original, self.columns(), self.index(), key) {
            joined_selected(&self.original, row, left, output, answer)?;
            emit.row(context, answer)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
