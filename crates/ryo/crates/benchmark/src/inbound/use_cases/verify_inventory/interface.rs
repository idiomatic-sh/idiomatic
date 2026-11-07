//! One complete inventory operation, with strict all-rule success.
use super::types::InventoryVerdict;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;
/// Verify all declared defaults and preserve their outcomes.
pub trait VerifyInventoryUseCase {
    /// Pass only when every pinned rule has executed passing evidence.
    ///
    /// # Errors
    /// Refuses malformed registrations or failed artifact persistence.
    fn verify_inventory(&self, registration: &Path, output: &Path) -> Result<InventoryVerdict, ReportsError>;
}
