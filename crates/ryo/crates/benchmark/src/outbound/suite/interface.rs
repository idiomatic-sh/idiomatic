//! Load and validate the suite's external artifacts.
use super::types::ArtifactPath;
use super::types::ArtifactSet;
use crate::outbound::suite::constants;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::Suite;
use ryo_benchmark_contracts::domain::workloads::types::Tool;
/// Read external suite artifacts and audit their authored default coverage.
pub trait SuitePort {
    /// Reverify declared physical observations before each contestant prepares its input.
    /// Legacy adapters may handle unspecified metadata; they cannot certify explicit observations.
    fn validate_file_metadata(
        &self,
        request: &ryo_benchmark_contracts::domain::requests::types::RunRequest,
    ) -> Result<(), Error> {
        if request.file_metadata.is_some() {
            Err(Error::Invalid(
                constants::FILE_METADATA_VERIFICATION_IS_UNAVAILABLE.into(),
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
            constants::CASE_SOURCE_READER_IS_UNAVAILABLE.into(),
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
            constants::ARTIFACT_READER_IS_UNAVAILABLE.into(),
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
            constants::CASE_OBSERVATION_STORE_IS_UNAVAILABLE.into(),
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
        owners: &[ryo::linter::RuleId],
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
    /// Seal the actual requested/auxiliary corpus bytes before both disk contestants.
    /// # Errors
    /// Refuses missing, escaping, symlinked or changed inputs without inventing metadata.
    fn seal_context(
        &self,
        root: ArtifactPath<'_>,
        sources: &[ryo::linter::RelativePath],
        tracked: &[std::path::PathBuf],
    ) -> Result<ryo_benchmark_contracts::domain::context::types::ContextManifest, Error>;
    /// Materialize only the explicitly authored subset in an owned empty directory.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid destination or failed copying of audited rule artifacts.
    fn stage(
        &self,
        directory: ArtifactPath<'_>,
        destination: ArtifactPath<'_>,
        authored: &[ryo::linter::RuleId],
        definitions: &std::collections::BTreeSet<ryo::linter::RuleId>,
        purpose: ryo_benchmark_contracts::domain::workloads::types::StagePurpose,
    ) -> Result<(), Error>;
    /// Read rule directory identities without deciding which rules own evidence.
    /// # Errors
    /// Refuses unreadable directories and malformed rule identities.
    fn rule_ids(&self, rules: ArtifactPath<'_>) -> Result<Vec<ryo::linter::RuleId>, Error>;

    /// Read an optional exact query; absence remains distinct from empty source.
    /// # Errors
    /// Refuses unreadable query bytes and invalid UTF8.
    fn rule_query(
        &self,
        rules: ArtifactPath<'_>,
        rule: &ryo::linter::RuleId,
    ) -> Result<Option<String>, Error>;

    /// Decode one owner's case declarations without deciding polarity or coverage.
    /// # Errors
    /// Refuses missing or malformed case data.
    fn rule_cases(
        &self,
        rules: ArtifactPath<'_>,
        rule: &ryo::linter::RuleId,
    ) -> Result<ryo_benchmark_contracts::domain::workloads::types::Cases, Error>;

    /// Verify physical module paths, symlink boundaries and readable source bytes.
    /// # Errors
    /// Refuses unsafe, missing or unreadable declared modules.
    fn verify_query_modules(&self, profile: ArtifactPath<'_>, tool: &Tool) -> Result<(), Error>;

    /// Materialize only sources already selected by application preparation.
    /// # Errors
    /// Refuses escaping sources and physical context contradictions.
    fn materialize_case(
        &self,
        destination: ArtifactPath<'_>,
        case: &ryo_benchmark_contracts::domain::workloads::types::Case,
    ) -> Result<super::types::MaterializedCase, Error>;

    /// Check and materialize Given entries after request configuration is admitted.
    /// # Errors
    /// Refuses entry collisions and unsupported physical entry kinds.
    fn materialize_inventory(
        &self,
        root: ArtifactPath<'_>,
        inventory: &ryo::linter::GivenProjectInventory,
    ) -> Result<(), Error>;
}
