//! Compose a boundary-owned registration and raw bytes with the existing shared validator.
use super::interface::ValidateCaseReceiptsUseCase;
use crate::{
    application::run::receipt_validation,
    domain::{diagnostics::errors::DiagnosticsError as Error, suite::process::types::ReceiptAudit},
    outbound::inventory::interface::InventoryPort,
};
use std::{path::Path, sync::Arc};
/// The edge selects the artifact store; application code does not hardwire it.
pub struct ValidateCaseReceiptsUseCaseImpl {
    store: Arc<dyn InventoryPort>,
}
impl ValidateCaseReceiptsUseCaseImpl {
    /// Inject the existing inventory artifact boundary.
    #[must_use]
    pub fn new(store: Arc<dyn InventoryPort>) -> Self {
        Self { store }
    }
}
impl std::fmt::Debug for ValidateCaseReceiptsUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
impl ValidateCaseReceiptsUseCase for ValidateCaseReceiptsUseCaseImpl {
    fn validate_case_receipts(
        &self,
        registration: &Path,
        receipt: &Path,
    ) -> Result<ReceiptAudit, Error> {
        let registration = self.store.load(registration).map_err(Error::Invalid)?;
        let captured = self.store.observations(receipt).map_err(Error::Invalid)?;
        receipt_validation::validate(&registration, &captured)
    }
}
