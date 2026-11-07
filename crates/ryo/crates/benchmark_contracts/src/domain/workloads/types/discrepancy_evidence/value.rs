//! The existing value and its behaviour, owned together.
use crate::domain::artifacts::types::digest::value::ArtifactDigest;
use std::path::PathBuf;
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
        language_version: String,
        /// Parser target supported by this evidence.
        target_configuration: String,
        /// Connection between observation and target.
        version_rationale: String,
    },
    /// A pinned previously published public tool contract.
    PublicToolContract {
        /// Safe profile-relative contract snapshot.
        path: PathBuf,
        /// Exact snapshot digest.
        sha256: ArtifactDigest,
        /// Published source revision.
        source_revision: String,
        /// Public source path.
        source_path: String,
        /// Case-specific policy explanation.
        rationale: String,
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
        witness_case: String,
        /// Why the separate control proves this selection boundary.
        rationale: String,
    },
}

impl DiscrepancyEvidence {
    /// The profile-contained artifact identity common to every authority.
    #[must_use]
    pub fn artifact(&self) -> (&PathBuf, &ArtifactDigest) {
        match self {
            Self::Language { path, sha256, .. }
            | Self::PublicToolContract { path, sha256, .. }
            | Self::PriorWorkerRun { path, sha256, .. } => (path, sha256),
        }
    }
}
