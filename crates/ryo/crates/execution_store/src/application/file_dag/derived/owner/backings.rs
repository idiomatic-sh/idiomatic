//! The owner consumes immutable physical allocation facts from opaque payloads.
use super::super::ledger::types::Backing;
use super::*;
pub(super) fn view(storage: &Arc<DerivedStorage>) -> Result<Arc<[Backing]>, OperatingError> {
    DerivedStorage::backings(storage)
}
