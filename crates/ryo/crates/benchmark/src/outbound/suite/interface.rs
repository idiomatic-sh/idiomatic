//! Load and validate the suite's external artifacts.
use crate::domain::suite::types::Audit;
use crate::domain::{
    diagnostics::errors::Error,
    suite::types::{Suite, Tool},
};
use std::path::Path;
/// Read external suite artifacts and audit their authored default coverage.
pub trait SuitePort {
    /// Decode the pinned linter profile below its artifact directory.
    fn profile(&self, directory: &Path) -> Result<Tool, Error>;
    /// Decode the shared language project manifest below its artifact directory.
    fn projects(&self, directory: &Path) -> Result<Suite, Error>;
    /// Check the executable's exact bytes and reported version against its profile.
    fn verify_tool(&self, executable: &Path, tool: &Tool) -> Result<(), Error>;
    /// Hash exact regular-file bytes and names, recursively for artifact directories.
    fn fingerprint(&self, paths: &[std::path::PathBuf]) -> Result<String, Error>;
    /// Materialize only the explicitly authored subset in an owned empty directory.
    fn stage(&self, directory: &Path, destination: &Path, audit: &Audit) -> Result<(), Error>;

    /// Validate authored queries and cases against the pinned default inventory.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid artifacts, unsupported rule identities or unreadable inputs.
    fn audit(&self, directory: &Path, tool: &Tool) -> Result<Audit, Error>;
}
