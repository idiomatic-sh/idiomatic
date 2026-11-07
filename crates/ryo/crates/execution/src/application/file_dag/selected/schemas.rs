//! The one place a selected population is paired with its equality index: the index kind is checked here.
use super::SelectedInput;
use crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::SelectedRows;

pub(in crate::application::file_dag) fn from_derived<'f>(
    original: &SelectedRows<'f>,
    index: AcquiredView,
) -> Result<SelectedInput<'f>, OperatingError> {
    if !matches!(
        index.view()?,
        ryo_derived::domain::contract::types::DerivedView::Equality(_)
    ) {
        return Err(OperatingError::WrongOperation);
    }
    Ok(SelectedInput {
        original: original.clone(),
        index,
    })
}
