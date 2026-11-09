//! Thin composition of external registration with the shared inventory loop.
use super::interface::VerifyInventoryUseCase;
use crate::application::inventory_verification::service as inventory;
use crate::domain::suite::types::inventory::value::InventoryVerdict;
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::inventory::interface::InventoryPort;
use crate::outbound::receipts::interface::ReceiptsPort;
use crate::outbound::suite::interface::SuitePort;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::{path::Path, sync::Arc};
/// The composition chooses the existing process adapter and its deadline.
pub struct VerifyInventoryUseCaseImpl {
    suite: Arc<dyn SuitePort>,
    receipts: Arc<dyn ReceiptsPort>,
    execution: Arc<dyn ExecutionPort>,
    store: Arc<dyn InventoryPort>,
}
impl VerifyInventoryUseCaseImpl {
    /// Bind the existing shared execution contract.
    #[must_use]
    pub fn new(
        suite: Arc<dyn SuitePort>,
        receipts: Arc<dyn ReceiptsPort>,
        execution: Arc<dyn ExecutionPort>,
        store: Arc<dyn InventoryPort>,
    ) -> Self {
        Self {
            suite,
            receipts,
            execution,
            store,
        }
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
    ) -> Result<InventoryVerdict, ReportsError> {
        let registration = self.store.load(registration)?;
        inventory::verify(
            &registration,
            output,
            self.suite.as_ref(),
            self.receipts.as_ref(),
            self.execution.as_ref(),
            self.store.as_ref(),
        )
    }
}
