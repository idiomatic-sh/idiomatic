//! Thin composition of external registration with the shared inventory loop.
use super::interface::VerifyInventoryUseCase;
use crate::{
    application::run::inventory,
    domain::diagnostics::errors::DiagnosticsError,
    outbound::{execution::interface::ExecutionPort, inventory::interface::InventoryPort},
};
use std::{path::Path, sync::Arc};
/// The composition chooses the existing process adapter and its deadline.
pub struct VerifyInventoryUseCaseImpl {
    execution: Arc<dyn ExecutionPort>,
    store: Arc<dyn InventoryPort>,
}
impl VerifyInventoryUseCaseImpl {
    /// Bind the existing shared execution contract.
    #[must_use]
    pub fn new(execution: Arc<dyn ExecutionPort>, store: Arc<dyn InventoryPort>) -> Self {
        Self { execution, store }
    }
}
impl std::fmt::Debug for VerifyInventoryUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
impl VerifyInventoryUseCase for VerifyInventoryUseCaseImpl {
    fn verify_inventory(
        &self,
        registration: &Path,
        output: &Path,
    ) -> Result<bool, DiagnosticsError> {
        let registration = self
            .store
            .load(registration)
            .map_err(DiagnosticsError::Invalid)?;
        inventory::verify(
            &registration,
            output,
            self.execution.as_ref(),
            self.store.as_ref(),
        )
    }
}
