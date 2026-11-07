//! Per-operation artifact identities, independent of adapter configuration.
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::documents::types::text::{CaseName, PathText, RuleName, Sha256Text};
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::process::types::ProcessFailure;
use ryo_benchmark_contracts::domain::reports::types::SourceByteOffset;
use ryo_benchmark_contracts::domain::workloads::types::discrepancy_evidence::value::{
    ContractSourcePath, LanguageVersion, ParserTarget, Rationale, SourceRevision, WitnessCase,
};
use ryo_benchmark_contracts::domain::workloads::types::tool_name::value::ToolName;
use ryo_benchmark_contracts::domain::workloads::types::tool_version::value::ToolVersion;
use ryo_benchmark_contracts::domain::workloads::types::{DiscrepancyBasis, DiscrepancyReason};
use std::path::PathBuf;
/// A source, destination or executable artifact selected for this operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactPath<'a>(pub &'a std::path::Path);
/// Exact artifact paths whose names and bytes participate in one fingerprint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactSet<'a>(pub &'a [std::path::PathBuf]);

/// Text an operation writes, or names a new artifact by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactText<'a>(pub &'a str);
/// The exact bytes of one artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactBytes(pub Vec<u8>);
/// The SHA256 of the sources of one case, as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceDigest(pub String);
/// One name a selection file lists.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SelectedName(pub String);
/// The exact RyoQL source of one rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleQuery(pub String);
/// The total size in bytes of a set of artifacts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactSize(pub u64);
/// A directory the adapter made and removes, with everything beneath it, when this value drops.
pub struct ScratchDirectory {
    /// Where the directory lives while the value is held.
    pub path: std::path::PathBuf,
    /// What the adapter holds to remove the directory.
    pub removal: ScratchRemoval,
}

/// The adapter's own handle on a directory's removal; it is opaque to every caller.
pub struct ScratchRemoval(pub Box<dyn Send + Sync>);

impl std::fmt::Debug for ScratchDirectory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

/// Where a run appends its raw case receipts, as the edge chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaseReceipts {
    /// The run records no case receipts.
    Unrecorded,
    /// The run appends its case receipts to this file.
    Recorded(std::path::PathBuf),
}

/// The context a materialized case was sealed under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MaterializedContext {
    /// The case runs on its own files.
    Isolated,
    /// The exact sealed disk dependency context that was materialized.
    Sealed(ryo_benchmark_contracts::domain::context::types::ContextManifest),
}

/// Observed physical addresses; this carries no audit or rule-selection authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterializedCase {
    /// Corpus root used for both contestants.
    pub root: std::path::PathBuf,
}

/// The identities one reference discrepancy certificate names, as authored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClaimedMetadata {
    /// The certificate names no declared file metadata.
    Undeclared,
    /// The certificate names this declared file metadata digest.
    Declared(Sha256Text),
}

/// One complete explicitly identified diagnostic a certificate claims the reference reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimedFinding {
    /// Rule identity, never inherited from an enclosing rule.
    pub rule: RuleName,
    /// Source-relative path.
    pub file: PathText,
    /// UTF8 byte start.
    pub start: SourceByteOffset,
    /// UTF8 byte end.
    pub end: SourceByteOffset,
}

/// Exact certificate identities, validated by application policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimedIdentity {
    /// Separate case metadata identity, mandatory when observations are declared.
    pub file_metadata_sha256: ClaimedMetadata,
    /// Reference name.
    pub tool: ToolName,
    /// Pinned version.
    pub version: ToolVersion,
    /// Originally observed approved native executable.
    pub observed_executable_sha256: Sha256Text,
    /// Exact profile bytes.
    pub profile_sha256: Sha256Text,
    /// Normalized effective invocation.
    pub settings_sha256: Sha256Text,
    /// Source filenames and bytes.
    pub source_sha256: Sha256Text,
    /// Complete reference expectation.
    pub reference_output_sha256: Sha256Text,
    /// Intended candidate expectation.
    pub intended_output_sha256: Sha256Text,
}

/// Independent artifact observation a certificate cites.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClaimedEvidence {
    /// Versioned language specification.
    LanguageSpec {
        /// Profile-relative artifact path.
        path: PathBuf,
        /// Exact artifact bytes.
        sha256: Sha256Text,
        /// Observed language version.
        language_version: LanguageVersion,
        /// Explicit parser contract.
        target_configuration: ParserTarget,
        /// Explanation connecting the observation to the target.
        version_rationale: Rationale,
    },
    /// Independently executed language implementation.
    IndependentLanguageRun {
        /// Profile-relative artifact path.
        path: PathBuf,
        /// Exact artifact bytes.
        sha256: Sha256Text,
        /// Observed language version.
        language_version: LanguageVersion,
        /// Explicit parser contract.
        target_configuration: ParserTarget,
        /// Explanation connecting the observation to the target.
        version_rationale: Rationale,
    },
    /// The exact prior public API contract whose policy is asserted.
    PublicToolContract {
        /// Profile-relative snapshot path.
        path: PathBuf,
        /// Exact snapshot bytes.
        sha256: Sha256Text,
        /// Published source revision.
        source_revision: SourceRevision,
        /// Public source path within that revision.
        source_path: ContractSourcePath,
        /// What the pinned text requires for this case.
        rationale: Rationale,
    },
    /// An actual prior accepted worker exercising the same selection boundary.
    PriorWorkerRun {
        /// Profile-relative complete observation path.
        path: PathBuf,
        /// Exact observation bytes.
        sha256: Sha256Text,
        /// Sealed prior worker identity.
        worker_sha256: Sha256Text,
        /// Exact input bytes of the separate control.
        control_source_sha256: Sha256Text,
        /// Exact control query identity.
        control_query_sha256: Sha256Text,
        /// Exact control profile identity.
        control_profile_sha256: Sha256Text,
        /// Named completed control that must exhibit the prior policy.
        witness_case: WitnessCase,
        /// How this control proves the same policy boundary without claiming identical rule input.
        rationale: Rationale,
    },
}

/// Whether a certificate claims an explicit process failure in place of diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClaimedProcess {
    /// The reference reports diagnostics.
    Absent,
    /// The reference fails with this explicitly evidenced process failure.
    Failure(ProcessFailure),
}

/// A reference-specific certificate read from an authored case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimedDiscrepancy {
    /// Explicit process expectation; absent preserves the existing diagnostic contract.
    pub process: ClaimedProcess,
    /// Exact reference findings.
    pub expected: Vec<ClaimedFinding>,
    /// Immutable certificate inputs.
    pub identity: ClaimedIdentity,
    /// Independent evidence files.
    pub evidence: Vec<ClaimedEvidence>,
    /// Authored explanation.
    pub reason: DiscrepancyReason,
}

/// The certificate of one authored case, with the authority it asserts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseClaim {
    /// Enclosing rule directory name.
    pub rule: RuleName,
    /// Authored case name.
    pub name: CaseName,
    /// The asserted authority for this exact two-sided difference.
    pub basis: DiscrepancyBasis,
    /// The independently evidenced reference expectation.
    pub discrepancy: ClaimedDiscrepancy,
}

/// Explicit artifact content to verify inside a physical profile boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceArtifact {
    /// Boundary-relative artifact name.
    pub path: PathBuf,
    /// Exact expected SHA256.
    pub sha256: ArtifactDigest,
    /// Exact authority-specific content the artifact must establish.
    pub requirement: EvidenceRequirement,
}

/// Evidence bytes are checked at the outbound boundary, not trusted from authored metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvidenceRequirement {
    /// Existing independent language artifact.
    Language,
    /// Complete published contract snapshot and its asserted source identity.
    PublicToolContract {
        /// Published source revision.
        source_revision: SourceRevision,
        /// Public source path within that revision.
        source_path: ContractSourcePath,
    },
    /// Complete prior worker observation and the selected positive witness.
    PriorWorkerRun {
        /// Sealed prior worker identity.
        worker_sha256: ArtifactDigest,
        /// Exact control input identity.
        control_source_sha256: ArtifactDigest,
        /// Exact control query identity.
        control_query_sha256: ArtifactDigest,
        /// Exact control profile identity.
        control_profile_sha256: ArtifactDigest,
        /// Named completed control that must exhibit the prior policy.
        witness_case: WitnessCase,
    },
}

/// The declared file metadata digest of a case, borrowed for one verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactMetadata<'a> {
    /// The case declares no file metadata.
    Undeclared,
    /// The case declares file metadata under this digest.
    Declared(&'a ArtifactDigest),
}

/// IO request containing no semantic case or language-policy types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaseArtifacts<'a> {
    /// Profile/evidence boundary.
    pub profile: &'a std::path::Path,
    /// Materialized source boundary.
    pub root: &'a std::path::Path,
    /// Ordered source-relative filenames.
    pub files: &'a [std::path::PathBuf],
    /// Exact filename/source digest.
    pub source_sha256: &'a ArtifactDigest,
    /// Exact declared observation subset, independently bounded by `files`.
    pub file_metadata_files: &'a [std::path::PathBuf],
    /// The exact digest of the declared actual file observations.
    pub file_metadata: ArtifactMetadata<'a>,
    /// Explicit independently authored evidence artifacts.
    pub evidence: &'a [EvidenceArtifact],
}

/// Captured verification-only bytes to persist beside one receipt file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapturedProcess<'a> {
    /// Collision-safe key framed from rule/case/side by the application.
    pub key: &'a ArtifactDigest,
    /// Actual root/files/normalized settings/executable/status provenance.
    pub invocation: &'a serde_json::Value,
    /// Exact unmodified stdout.
    pub stdout: &'a CapturedBytes,
    /// Exact unmodified stderr.
    pub stderr: &'a CapturedBytes,
}
