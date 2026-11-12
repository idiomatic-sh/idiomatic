//! The `backings` application operation.
use crate::application::file_dag::derived::owner::acquisition::semantics::storage::service::DerivedStorage;
use std::sync::Arc;

use crate::application::file_dag::accounting::ledger::types::Backing;
use ryo_execution_domain::domain::operating::errors::OperatingError;

pub(crate) fn view(storage: &Arc<DerivedStorage>) -> Result<Arc<[Backing]>, OperatingError> {
    DerivedStorage::backings(storage)
}
