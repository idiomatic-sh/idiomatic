//! Generation-local selected-row index and direct projected-cell emission.
//! Only the application retains selection metadata; primitives receive borrowed typed rows.

use super::derived::owner::acquisition::admission::service::AcquiredView;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::views::{IndexedRows};
use crate::domain::table_model::types::RepresentationInput as BorrowedRows;
use crate::domain::table_model::types::SelectedRows;
use ryo_derived::domain::equality::types::KeyIndex;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// One complete selected population and an index over only its declared key cells.
/// The source remains borrowed from this physical file; metadata is an Arc clone.
pub(crate) struct SelectedInput<'f> {
    original: SelectedRows<'f>,
    index: AcquiredView,
}
impl<'f> SelectedInput<'f> {
    fn columns(&self) -> &[ColumnIndex] {
        let columns = match &self.index.contract().request {
            crate::domain::derived_views::types::ViewRequest::Equality { columns } => Some(columns),
            _ => None,
        };
        columns.expect(constants::PREPARED_EQUALITY)
    }
    fn index(&self) -> &KeyIndex {
        let ryo_derived::domain::contract::types::DerivedView::Equality(index) =
            self.index.view().expect(constants::VALIDATED_EQUALITY)
        else {
            unreachable!("validated equality view")
        };
        index
    }
    pub(crate) fn view(&self) -> IndexedRows<'_> {
        IndexedRows {
            #[cfg(feature = "diagnostic-row-progress")]
            progress: crate::application::file_dag::route_counts::progress::service::progress,
            rows: BorrowedRows::Selected {
                source: self.original.source,
                positions: crate::domain::stored_selection::types::BorrowedPositions(
                    &self.original.selection.rows,
                ),
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
mod constants;
pub(in crate::application::file_dag) mod schemas;

use ryo_derived::domain::request::types::DerivedKind;
