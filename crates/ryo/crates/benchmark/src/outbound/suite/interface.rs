//! Load and validate the suite's external artifacts.
use super::types::{ArtifactPath, ArtifactSet};
use crate::domain::suite::types::audit::value::Audit;
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    suite::types::{ArtifactDigest, Suite, Tool},
};
/// Read external suite artifacts and audit their authored default coverage.
pub trait SuitePort {
    /// Decode the pinned linter profile below its artifact directory.
    fn profile(&self, directory: ArtifactPath<'_>) -> Result<Tool, Error>;
    /// Decode the shared language project manifest below its artifact directory.
    fn projects(&self, directory: ArtifactPath<'_>) -> Result<Suite, Error>;
    /// Check the executable's exact bytes and reported version against its profile.
    fn verify_tool(&self, executable: ArtifactPath<'_>, tool: &Tool) -> Result<(), Error>;
    /// Hash exact regular-file bytes and names, recursively for artifact directories.
    fn fingerprint(&self, paths: ArtifactSet<'_>) -> Result<ArtifactDigest, Error>;
    /// Hash one regular file's exact bytes with unframed SHA256 for executable pinning.
    fn digest(&self, path: ArtifactPath<'_>) -> Result<ArtifactDigest, Error>;
    /// Freeze the shared project manifest and every declared patch in an owned directory.
    fn stage_projects(
        &self,
        source: ArtifactPath<'_>,
        destination: ArtifactPath<'_>,
    ) -> Result<(), Error>;
    /// Materialize only the explicitly authored subset in an owned empty directory.
    fn stage(
        &self,
        directory: ArtifactPath<'_>,
        destination: ArtifactPath<'_>,
        audit: &Audit,
    ) -> Result<(), Error>;
    /// Materialize the snapshot's independent case sources without executing contestants.
    fn cases(
        &self,
        rules: ArtifactPath<'_>,
        scratch: ArtifactPath<'_>,
    ) -> Result<Vec<crate::domain::suite::types::PreparedCase>, Error>;

    /// Validate authored queries and cases against the pinned default inventory.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid artifacts, unsupported rule identities or unreadable inputs.
    fn audit(&self, directory: ArtifactPath<'_>, tool: &Tool) -> Result<Audit, Error>;
}
