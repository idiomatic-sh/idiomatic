//! Artifact-bound wire data; no application or domain dependencies.
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// A reference-specific expectation read from an authored case.
#[derive(Clone, Debug)]
pub struct CaseReferenceWire {
    /// Enclosing rule directory name.
    pub rule: String,
    /// Authored case name.
    pub name: String,
    /// The asserted authority for this exact two-sided difference.
    pub basis: DiscrepancyBasisWire,
    /// Raw independently evidenced reference expectation.
    pub discrepancy: ReferenceDiscrepancyWire,
}
/// A language correction and a documented tool policy difference are distinct claims.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscrepancyBasisWire {
    /// Independent language meaning contradicts the reference output.
    LanguageDefect,
    /// A prior public tool contract intentionally differs from the reference tool's selection.
    ToolContract,
}
/// Serialized reference contract; semantic conversion belongs to the application.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ReferenceDiscrepancyWire {
    /// Explicit process expectation; absent preserves the existing diagnostic contract.
    #[serde(default)]
    pub process: Option<super::process::ProcessFailureWire>,
    /// Exact reference findings.
    #[serde(default)]
    pub expected: Vec<ReferenceFindingWire>,
    /// Immutable certificate inputs.
    pub identity: ReferenceIdentityWire,
    /// Independent evidence files.
    pub evidence: Vec<EvidenceWire>,
    /// Authored explanation.
    pub reason: String,
}
/// One complete explicitly identified diagnostic.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceFindingWire {
    /// Rule identity, never inherited from an enclosing rule.
    pub rule: String,
    /// Source-relative path.
    pub file: String,
    /// UTF8 byte start.
    pub start: usize,
    /// UTF8 byte end.
    pub end: usize,
}
/// Exact certificate identities, validated by application policy.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ReferenceIdentityWire {
    /// Separate optional case metadata identity, mandatory when observations are declared.
    #[serde(default)]
    pub file_metadata_sha256: Option<String>,
    /// Reference name.
    pub tool: String,
    /// Pinned version.
    pub version: String,
    /// Originally observed approved native executable.
    pub observed_executable_sha256: String,
    /// Exact profile bytes.
    pub profile_sha256: String,
    /// Normalized effective invocation.
    pub settings_sha256: String,
    /// Source filenames and bytes.
    pub source_sha256: String,
    /// Complete reference expectation.
    pub reference_output_sha256: String,
    /// Intended candidate expectation.
    pub intended_output_sha256: String,
}
/// Independent artifact observation.
#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case",
    deny_unknown_fields
)]
pub enum EvidenceWire {
    /// Versioned language specification.
    LanguageSpec {
        /// Profile-relative artifact path.
        path: PathBuf,
        /// Exact artifact bytes.
        sha256: String,
        /// Observed language version.
        language_version: String,
        /// Explicit parser contract.
        target_configuration: String,
        /// Explanation connecting the observation to the target.
        version_rationale: String,
    },
    /// Independently executed language implementation.
    IndependentLanguageRun {
        /// Profile-relative artifact path.
        path: PathBuf,
        /// Exact artifact bytes.
        sha256: String,
        /// Observed language version.
        language_version: String,
        /// Explicit parser contract.
        target_configuration: String,
        /// Explanation connecting the observation to the target.
        version_rationale: String,
    },
    /// The exact prior public API contract whose policy is asserted.
    PublicToolContract {
        /// Profile-relative snapshot path.
        path: PathBuf,
        /// Exact snapshot bytes.
        sha256: String,
        /// Published source revision.
        source_revision: String,
        /// Public source path within that revision.
        source_path: String,
        /// What the pinned text requires for this case.
        rationale: String,
    },
    /// An actual prior accepted worker exercising the same selection boundary.
    PriorWorkerRun {
        /// Profile-relative complete observation path.
        path: PathBuf,
        /// Exact observation bytes.
        sha256: String,
        /// Sealed prior worker identity.
        worker_sha256: String,
        /// Exact input bytes of the separate control.
        control_source_sha256: String,
        /// Exact control query identity.
        control_query_sha256: String,
        /// Exact control profile identity.
        control_profile_sha256: String,
        /// Named completed control that must exhibit the prior policy.
        witness_case: String,
        /// How this control proves the same policy boundary without claiming identical rule input.
        rationale: String,
    },
}
impl EvidenceWire {
    /// The exact profile-contained artifact shared by all evidence kinds.
    #[must_use]
    pub fn artifact(&self) -> (&Path, &str) {
        match self {
            Self::LanguageSpec { path, sha256, .. }
            | Self::IndependentLanguageRun { path, sha256, .. }
            | Self::PublicToolContract { path, sha256, .. }
            | Self::PriorWorkerRun { path, sha256, .. } => (path, sha256),
        }
    }
    /// The content contract to check after the artifact hash is verified.
    #[must_use]
    pub fn requirement(&self) -> EvidenceRequirement {
        match self {
            Self::LanguageSpec { .. } | Self::IndependentLanguageRun { .. } => {
                EvidenceRequirement::Language
            }
            Self::PublicToolContract {
                source_revision,
                source_path,
                ..
            } => EvidenceRequirement::PublicToolContract {
                source_revision: source_revision.clone(),
                source_path: source_path.clone(),
            },
            Self::PriorWorkerRun {
                worker_sha256,
                control_source_sha256,
                control_query_sha256,
                control_profile_sha256,
                witness_case,
                ..
            } => EvidenceRequirement::PriorWorkerRun {
                worker_sha256: worker_sha256.clone(),
                control_source_sha256: control_source_sha256.clone(),
                control_query_sha256: control_query_sha256.clone(),
                control_profile_sha256: control_profile_sha256.clone(),
                witness_case: witness_case.clone(),
            },
        }
    }
}
/// Explicit artifact content to verify inside a physical profile boundary.
#[derive(Clone, Debug)]
pub struct EvidenceArtifact {
    /// Boundary-relative artifact name.
    pub path: PathBuf,
    /// Exact expected SHA256.
    pub sha256: String,
    /// Exact authority-specific content the artifact must establish.
    pub requirement: EvidenceRequirement,
}
/// Evidence bytes are checked at the outbound boundary, not trusted from authored metadata.
#[derive(Clone, Debug)]
pub enum EvidenceRequirement {
    /// Existing independent language artifact.
    Language,
    /// Complete published contract snapshot and its asserted source identity.
    PublicToolContract {
        source_revision: String,
        source_path: String,
    },
    /// Complete prior worker observation and the selected positive witness.
    PriorWorkerRun {
        worker_sha256: String,
        control_source_sha256: String,
        control_query_sha256: String,
        control_profile_sha256: String,
        witness_case: String,
    },
}
/// IO request containing no semantic case or language-policy types.
#[derive(Clone, Copy, Debug)]
pub struct CaseArtifacts<'a> {
    /// Profile/evidence boundary.
    pub profile: &'a Path,
    /// Materialized source boundary.
    pub root: &'a Path,
    /// Ordered source-relative filenames.
    pub files: &'a [PathBuf],
    /// Exact filename/source digest.
    pub source_sha256: &'a str,
    /// Exact declared observation subset, independently bounded by `files`.
    pub file_metadata_files: &'a [PathBuf],
    /// Optional exact digest of the declared actual file observations.
    pub file_metadata_sha256: Option<&'a str>,
    /// Explicit independently authored evidence artifacts.
    pub evidence: &'a [EvidenceArtifact],
}
/// IO-boundary failure, independent of domain diagnostics.
#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    /// Artifact IO failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Authored wire data could not be decoded.
    #[error(transparent)]
    Decode(#[from] toml::de::Error),
    /// Artifact identity or boundary mismatch.
    #[error("{0}")]
    Invalid(String),
}
