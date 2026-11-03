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
    /// Raw independently evidenced reference expectation.
    pub discrepancy: ReferenceDiscrepancyWire,
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
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct EvidenceWire {
    /// Profile-relative artifact path.
    pub path: PathBuf,
    /// Exact artifact bytes.
    pub sha256: String,
    /// Independent authority category.
    pub kind: EvidenceKindWire,
    /// Observed language version.
    pub language_version: String,
    /// Explicit parser contract.
    pub target_configuration: String,
    /// Explanation connecting the observation to the target.
    pub version_rationale: String,
}
/// Competitor output alone is not an independent authority.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceKindWire {
    /// Versioned language specification.
    LanguageSpec,
    /// Independently executed language implementation.
    IndependentLanguageRun,
}
/// Explicit artifact content to verify inside a physical profile boundary.
#[derive(Clone, Debug)]
pub struct EvidenceArtifact {
    /// Boundary-relative artifact name.
    pub path: PathBuf,
    /// Exact expected SHA256.
    pub sha256: String,
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
