//! Typed equality indexing over existing bulk layouts; no acquisition or retention policy.
use super::interface::DerivedViewsItem;
use crate::domain::{
    derived_views::{service, types::*},
    operating::errors::OperatingError,
    operation_schema::types::InputSchema,
    table_model::{
        service::{index, index_selected_view, index_stored},
        types::{KeyIndex, On, SelectedView},
    },
};
pub(crate) struct EqualityItem;
impl DerivedViewsItem for EqualityItem {
    fn prepare(
        &self,
        request: &ViewRequest,
        schemas: &[InputSchema<'_>],
    ) -> Result<DerivedContract, OperatingError> {
        service::prepare(DerivedKind::Equality, request, schemas)
    }
    fn build(
        &self,
        contract: &DerivedContract,
        inputs: DerivedInputs<'_, '_>,
        storage: &mut DerivedStorage,
        _scratch: &mut DerivedScratch,
    ) -> Result<(), OperatingError> {
        service::validate(DerivedKind::Equality, contract)?;
        let (ViewRequest::Equality { columns }, DerivedInputs::Rows(inputs)) =
            (&contract.request, inputs)
        else {
            return Err(OperatingError::WrongOperation);
        };
        let source = *inputs
            .first()
            .ok_or(OperatingError::MissingInput { position: 1 })?;
        let mut out = match std::mem::take(storage) {
            DerivedStorage::Equality(index) => index,
            _ => KeyIndex::default(),
        };
        match source {
            crate::domain::operator::contract::views::BorrowedRows::Stored(view) => {
                index_stored(view, columns, &mut out)?
            }
            crate::domain::operator::contract::views::BorrowedRows::Selected {
                source,
                positions,
                columns: projection,
                kinds,
            } => index_selected_view(
                SelectedView {
                    source,
                    positions,
                    columns: projection,
                    kinds,
                },
                columns,
                &mut out,
            )?,
            crate::domain::operator::contract::views::BorrowedRows::Computed(table) => {
                index(table, On::Columns(columns), &mut out)?
            }
            crate::domain::operator::contract::views::BorrowedRows::Tree(_) => {
                crate::domain::table_model::service::index_cells(
                    crate::domain::table_model::types::RowCount(source.count()),
                    columns,
                    &mut out,
                    |row, column| source.cell(row, column),
                );
            }
        }
        *storage = DerivedStorage::Equality(out);
        Ok(())
    }
    fn view<'a>(
        &self,
        contract: &DerivedContract,
        storage: &'a DerivedStorage,
    ) -> Result<DerivedView<'a>, OperatingError> {
        service::view(DerivedKind::Equality, contract, storage)
    }
}
