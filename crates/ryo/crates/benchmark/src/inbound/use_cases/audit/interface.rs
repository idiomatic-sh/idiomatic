//! Audit the exact shipped-default inventory.
use crate::domain::suite::types::audit::value::Audit;
use crate::domain::{diagnostics::errors::DiagnosticsError as Error, suite::types::Tool};
use std::path::Path;
/// Audit benchmark artifacts against a pinned default profile.
pub trait AuditUseCase {
    /// Load coverage evidence without claiming unimplemented defaults.
    ///
    /// # Errors
    ///
    /// Returns an error if the suite boundary cannot validate the artifacts.
    fn audit(&self, directory: &Path, tool: &Tool) -> Result<Audit, Error>;
}
