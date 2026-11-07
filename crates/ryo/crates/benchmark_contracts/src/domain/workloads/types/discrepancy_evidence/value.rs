//! The existing value and its behaviour, owned together.
use crate::domain::artifacts::types::digest::value::ArtifactDigest;
use std::path::PathBuf;
/// Observed version of an independent language implementation or specification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageVersion(
    /// The version text.
    pub String,
);

/// Parser target supported by a piece of discrepancy evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParserTarget(
    /// The parser target text.
    pub String,
);

/// Why a piece of evidence supports a discrepancy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rationale(
    /// The explanation.
    pub String,
);

/// Published source revision of a public tool contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRevision(
    /// The revision identifier.
    pub String,
);

/// Public source path of a tool contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractContractSourcePath(
    /// The path as published.
    pub String,
);

/// Named completed control that exhibits a prior policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WitnessCase(
    /// The control case's name.
    pub String,
);

/// The profile-contained artifact of a piece of evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactIdentity<'a> {
    /// Safe profile-relative artifact path.
    pub path: &'a PathBuf,
    /// Exact artifact content digest.
    pub sha256: &'a ArtifactDigest,
}

/// Independent authority accepted for a reference discrepancy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscrepancyEvidenceKind {
    /// A versioned language specification artifact.
    LanguageSpec,
    /// An independent language implementation observation.
    IndependentLanguageRun,
}

/// One exact artifact within the profile boundary, with evidence fields specific to its authority.
#[derive(Clone, Debug)]
pub enum DiscrepancyEvidence {
    /// Independent language meaning.
    Language {
        /// Safe profile-relative artifact path.
        path: PathBuf,
        /// Exact artifact content digest.
        sha256: ArtifactDigest,
        /// Language specification or execution.
        kind: DiscrepancyEvidenceKind,
        /// Observed language version.
        language_version: LanguageVersion,
        /// Parser target supported by this evidence.
        target_configuration: ParserTarget,
        /// Connection between observation and target.
        version_rationale: Rationale,
    },
    /// A pinned previously published public tool contract.
    PublicToolContract {
        /// Safe profile-relative contract snapshot.
        path: PathBuf,
        /// Exact snapshot digest.
        sha256: ArtifactDigest,
        /// Published source revision.
        source_revision: SourceRevision,
        /// Public source path.
        source_path: ContractSourcePath,
        /// Case-specific policy explanation.
        rationale: Rationale,
    },
    /// Actual prior worker execution of the same policy boundary.
    PriorWorkerRun {
        /// Safe profile-relative complete observation.
        path: PathBuf,
        /// Exact observation digest.
        sha256: ArtifactDigest,
        /// Sealed worker identity.
        worker_sha256: ArtifactDigest,
        /// Exact control input, query and profile identities; these need not equal the rule case.
        control_source_sha256: ArtifactDigest,
        /// Exact control query identity.
        control_query_sha256: ArtifactDigest,
        /// Exact control profile identity.
        control_profile_sha256: ArtifactDigest,
        /// Named completed control that must exhibit the prior policy.
        witness_case: WitnessCase,
        /// Why the separate control proves this selection boundary.
        rationale: Rationale,
    },
}

impl DiscrepancyEvidence {
    /// The profile-contained artifact identity common to every authority.
    #[must_use]
    pub fn artifact(&self) -> ArtifactIdentity<'_> {
        match self {
            Self::Language { path, sha256, .. }
            | Self::PublicToolContract { path, sha256, .. }
            | Self::PriorWorkerRun { path, sha256, .. } => ArtifactIdentity { path, sha256 },
        }
    }
}
