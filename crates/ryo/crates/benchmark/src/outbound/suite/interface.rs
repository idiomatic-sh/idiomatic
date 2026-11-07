//! Load and validate the suite's external artifacts.
use super::types::{
    ArtifactBytes, ArtifactPath, ArtifactSet, ArtifactSize, ArtifactText, RuleQuery,
    RetainedDirectory, SelectedName, SourceDigest,
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::Suite;
use ryo_benchmark_contracts::domain::workloads::types::Tool;
/// A directory the adapter made and removes, with everything beneath it, when this handle drops.
pub trait ScopedDirectory: Send + Sync {
    /// Where the directory lives while the handle is held.
    fn path(&self) -> &std::path::Path;
}
/// Read external suite artifacts and audit their authored default coverage.
pub trait SuitePort {
    /// Reverify declared physical observations before each contestant prepares its input.
    ///
    /// # Errors
    /// Refuses drifted or unverifiable declared file metadata.
    fn validate_file_metadata(
        &self,
        request: &ryo_benchmark_contracts::domain::requests::types::RunRequest,
    ) -> Result<(), Error>;

    /// Read the exact declared source bytes using canonical filename framing.
    ///
    /// # Errors
    /// Refuses missing, unreadable or escaping sources.
    fn case_source_digest(
        &self,
        root: ArtifactPath<'_>,
        files: ArtifactSet<'_>,
    ) -> Result<SourceDigest, super::errors::SuiteError>;

    /// Read only explicitly named profile-relative artifact bytes.
    ///
    /// # Errors
    /// Refuses missing, stale, escaping or non-regular files.
    fn read_discrepancy_artifacts(
        &self,
        profile: ArtifactPath<'_>,
        artifacts: &[super::types::EvidenceArtifact],
    ) -> Result<
        std::collections::BTreeMap<std::path::PathBuf, ArtifactBytes>,
        super::errors::SuiteError,
    >;
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
    fn case_claims(
        &self,
        profile: ArtifactPath<'_>,
        owners: &[ryo::linter::RuleId],
    ) -> Result<
        Vec<super::types::CaseClaim>,
        super::errors::SuiteError,
    >;
    /// Validate explicit source/evidence bytes inside their physical boundaries.
    ///
    /// # Errors
    /// Refuses unreadable sources, escaping paths and stale artifact digests.
    fn verify_discrepancy_artifacts(
        &self,
        artifacts: super::types::CaseArtifacts<'_>,
    ) -> Result<(), super::errors::SuiteError>;
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
    /// Create a fresh directory beneath `parent`, or beneath the system temporary directory when
    /// the parent is the empty path; the adapter removes it when the returned value drops.
    ///
    /// # Errors
    /// Refuses a parent that cannot hold a new directory.
    fn scoped_directory(
        &self,
        parent: ArtifactPath<'_>,
        prefix: ArtifactText<'_>,
    ) -> Result<Box<dyn ScopedDirectory>, Error>;
    /// Create a fresh directory beneath `parent` that stays after the run, for the caller to
    /// inspect, and answer its path.
    ///
    /// # Errors
    /// Refuses a parent that cannot hold a new directory.
    fn retained_directory(
        &self,
        parent: ArtifactPath<'_>,
        prefix: ArtifactText<'_>,
    ) -> Result<RetainedDirectory, Error>;
    /// Copy one executable's exact bytes to another path.
    ///
    /// # Errors
    /// Refuses a missing source or an unwritable destination.
    fn copy_file(&self, from: ArtifactPath<'_>, to: ArtifactPath<'_>) -> Result<(), Error>;
    /// Create the directory and its parents, and answer its canonical path.
    ///
    /// # Errors
    /// Refuses a path that cannot be created or resolved.
    fn ensure_directory(&self, directory: ArtifactPath<'_>) -> Result<std::path::PathBuf, Error>;
    /// Write one text file.
    ///
    /// # Errors
    /// Refuses an unwritable path.
    fn write_text(&self, file: ArtifactPath<'_>, text: ArtifactText<'_>) -> Result<(), Error>;
    /// The names a selection file lists.
    ///
    /// # Errors
    /// Refuses a missing file or one that is not a list of names.
    fn selection(&self, file: ArtifactPath<'_>) -> Result<Vec<SelectedName>, Error>;
    /// The total size in bytes of these files.
    ///
    /// # Errors
    /// Refuses a file that cannot be inspected.
    fn total_size(&self, files: ArtifactSet<'_>) -> Result<ArtifactSize, Error>;
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
    ) -> Result<Option<RuleQuery>, Error>;

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
        context: &super::types::MaterializedContext,
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
