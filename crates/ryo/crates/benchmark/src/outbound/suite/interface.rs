//! Load and validate the suite's external artifacts.
use super::types::{ArtifactPath, ArtifactSet};
use crate::domain::suite::types::audit::value::Audit;
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    suite::types::{ArtifactDigest, Suite, Tool},
};
/// Read external suite artifacts and audit their authored default coverage.
pub trait SuitePort {
    /// Reverify declared physical observations before each contestant prepares its input.
    /// Legacy adapters may handle unspecified metadata; they cannot certify explicit observations.
    fn validate_file_metadata(
        &self,
        request: &crate::domain::suite::types::RunRequest,
    ) -> Result<(), Error> {
        if request.file_metadata.is_some() {
            Err(Error::Invalid(
                "file metadata verification is unavailable".into(),
            ))
        } else {
            Ok(())
        }
    }

    /// Optional receipt sink chosen at the outer edge; fixtures inject it without global state.
    fn case_receipt_file(&self) -> Option<std::path::PathBuf> {
        None
    }

    /// Read the exact declared source bytes using canonical filename framing.
    ///
    /// # Errors
    /// Refuses missing, unreadable or escaping sources.
    fn case_source_digest(
        &self,
        _root: ArtifactPath<'_>,
        _files: &[std::path::PathBuf],
    ) -> Result<String, super::types::discrepancy::ArtifactError> {
        Err(super::types::discrepancy::ArtifactError::Invalid(
            "case source reader is unavailable".into(),
        ))
    }

    /// Read only explicitly named profile-relative artifact bytes.
    ///
    /// # Errors
    /// Refuses missing, stale, escaping or non-regular files.
    fn read_discrepancy_artifacts(
        &self,
        _profile: ArtifactPath<'_>,
        _artifacts: &[super::types::discrepancy::EvidenceArtifact],
    ) -> Result<
        std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
        super::types::discrepancy::ArtifactError,
    > {
        Err(super::types::discrepancy::ArtifactError::Invalid(
            "artifact reader is unavailable".into(),
        ))
    }
    /// Append an application-owned observation and preserve optional raw process bytes.
    ///
    /// # Errors
    /// Refuses unsafe receipt names or failed artifact persistence.
    fn record_case(
        &self,
        _receipt: ArtifactPath<'_>,
        _record: serde_json::Value,
        _process: Option<super::types::process::CapturedProcess<'_>>,
    ) -> Result<(), super::types::discrepancy::ArtifactError> {
        Err(super::types::discrepancy::ArtifactError::Invalid(
            "case observation store is unavailable".into(),
        ))
    }

    /// Decode the pinned linter profile below its artifact directory.
    ///
    /// # Errors
    ///
    /// Returns an error for unreadable, malformed, or inconsistent profile artifacts.
    fn profile(&self, directory: ArtifactPath<'_>) -> Result<Tool, Error>;
    /// Decode the shared language project manifest below its artifact directory.
    ///
    /// # Errors
    ///
    /// Returns an error for unreadable or malformed project manifests.
    fn projects(&self, directory: ArtifactPath<'_>) -> Result<Suite, Error>;
    /// Check the executable's exact bytes and reported version against its profile.
    ///
    /// # Errors
    ///
    /// Returns an error for unreadable bytes, a digest mismatch, launch failure, or a version mismatch.
    fn verify_tool(&self, executable: ArtifactPath<'_>, tool: &Tool) -> Result<(), Error>;
    /// Hash exact regular-file bytes and names, recursively for artifact directories.
    ///
    /// # Errors
    ///
    /// Returns an error for unreadable artifacts or unsupported filesystem entries.
    fn fingerprint(&self, paths: ArtifactSet<'_>) -> Result<ArtifactDigest, Error>;
    /// Read optional reference expectation wire data from authored rule cases.
    ///
    /// # Errors
    /// Refuses unreadable or malformed artifacts; semantic conversion belongs to the application.
    fn case_reference_wires(
        &self,
        profile: ArtifactPath<'_>,
    ) -> Result<
        Vec<super::types::discrepancy::CaseReferenceWire>,
        super::types::discrepancy::ArtifactError,
    >;
    /// Validate explicit source/evidence bytes inside their physical boundaries.
    ///
    /// # Errors
    /// Refuses unreadable sources, escaping paths and stale artifact digests.
    fn verify_discrepancy_artifacts(
        &self,
        artifacts: super::types::discrepancy::CaseArtifacts<'_>,
    ) -> Result<(), super::types::discrepancy::ArtifactError>;
    /// Hash one regular file's exact bytes with unframed SHA256 for executable pinning.
    ///
    /// # Errors
    ///
    /// Returns an error when the path is not a readable regular file.
    fn digest(&self, path: ArtifactPath<'_>) -> Result<ArtifactDigest, Error>;
    /// Freeze the shared project manifest and every declared patch in an owned directory.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid patch paths, unreadable sources, or failed snapshot writes.
    fn stage_projects(
        &self,
        source: ArtifactPath<'_>,
        destination: ArtifactPath<'_>,
    ) -> Result<(), Error>;
    /// Materialize only the explicitly authored subset in an owned empty directory.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid destination or failed copying of audited rule artifacts.
    fn stage(
        &self,
        directory: ArtifactPath<'_>,
        destination: ArtifactPath<'_>,
        audit: &Audit,
    ) -> Result<(), Error>;
    /// Materialize the snapshot's independent case sources without executing contestants.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed cases, invalid source paths, or failed materialization.
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
