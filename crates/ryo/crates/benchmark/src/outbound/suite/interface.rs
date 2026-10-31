//! Load and validate the suite's external artifacts.
use crate::domain::suite::types::Audit;
use crate::domain::{diagnostics::errors::Error, suite::types::Tool};
use std::path::Path;
/// Read external suite artifacts and audit their authored default coverage.
pub trait SuitePort {
    /// Validate authored queries and cases against the pinned default inventory.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid artifacts, unsupported rule identities or unreadable inputs.
    fn audit(&self, directory: &Path, tool: &Tool) -> Result<Audit, Error>;
}
