//! Offline checks share the same policy as fresh inventory execution.
use crate::domain::{diagnostics::errors::DiagnosticsError, suite::process::types::ReceiptAudit};
use std::path::Path;
/// Validate one explicit receipt file, including shard-specific raw artifacts.
pub trait ValidateCaseReceiptsUseCase {
    /// Return all validated identities and explicit failures; no failed case is skipped.
    ///
    /// # Errors
    /// Refuses malformed registrations, escaping paths or unavailable raw evidence.
    fn validate_case_receipts(
        &self,
        registration: &Path,
        receipt: &Path,
    ) -> Result<ReceiptAudit, DiagnosticsError>;
}
