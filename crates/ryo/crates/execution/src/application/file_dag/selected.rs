//! Generation-local selected-row index and direct projected-cell emission.
//! Only the application retains selection metadata; primitives receive borrowed typed rows.

use super::derived::view::AcquiredView;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::views::{BorrowedRows, IndexedRows};
use crate::domain::table_model::types::SelectedRows;
use ryo_derived::domain::contract::types::KeyIndex;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// One complete selected population and an index over only its declared key cells.
/// The source remains borrowed from this physical file; metadata is an Arc clone.
pub(crate) struct SelectedInput<'f> {
    original: SelectedRows<'f>,
    index: AcquiredView,
}
impl<'f> SelectedInput<'f> {
    pub(super) fn from_derived(
        original: &SelectedRows<'f>,
        index: AcquiredView,
    ) -> Result<Self, OperatingError> {
        if !matches!(
            index.view()?,
            ryo_derived::domain::contract::types::DerivedView::Equality(_)
        ) {
            return Err(OperatingError::WrongOperation);
        }
        Ok(Self {
            original: original.clone(),
            index,
        })
    }
    fn columns(&self) -> &[ColumnIndex] {
        let crate::domain::derived_views::types::ViewRequest::Equality { columns } =
            &self.index.contract().request
        else {
            unreachable!("prepared equality")
        };
        columns
    }
    fn index(&self) -> &KeyIndex {
        let ryo_derived::domain::contract::types::DerivedView::Equality(index) =
            self.index.view().expect("validated equality view")
        else {
            unreachable!("validated equality view")
        };
        index
    }
    pub(crate) fn view(&self) -> IndexedRows<'_> {
        IndexedRows {
            #[cfg(feature = "diagnostic-row-progress")]
            progress: crate::application::file_dag::route_counts::progress,
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
}

#[cfg(test)]
mod tests;

use ryo_derived::domain::contract::types::DerivedKind;
