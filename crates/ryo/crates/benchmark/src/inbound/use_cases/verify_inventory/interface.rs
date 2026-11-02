//! One complete inventory operation, with strict all-rule success.
use crate::domain::diagnostics::errors::DiagnosticsError;
use std::path::Path;
/// Verify all declared defaults and preserve their outcomes.
pub trait VerifyInventoryUseCase {
    /// Return true only when every pinned rule has executed passing evidence.
    ///
    /// # Errors
    /// Refuses malformed registrations or failed artifact persistence.
    fn verify_inventory(
        &self,
        registration: &Path,
        output: &Path,
    ) -> Result<bool, DiagnosticsError>;
}
