//! Per-operation artifact identities, independent of adapter configuration.
/// A source, destination or executable artifact selected for this operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactPath<'a>(pub &'a std::path::Path);
/// Exact artifact paths whose names and bytes participate in one fingerprint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactSet<'a>(pub &'a [std::path::PathBuf]);

/// Observed physical addresses; this carries no audit or rule-selection authority.
pub struct MaterializedCase {
    /// Corpus root used for both contestants.
    pub root: std::path::PathBuf,
    /// Exact sealed disk dependency context when one was materialized.
    pub context_manifest: Option<ryo_benchmark_contracts::domain::context::types::ContextManifest>,
}

/// Reference evidence wire values owned by this boundary.
pub mod discrepancy;

/// Byte-exact expected process failure artifacts and templates.
pub mod process;

/// Named text and yes/no values of the receipt documents.
pub mod text;

/// One diagnostic as a document records it.
pub mod finding;

/// A documented difference from the reference tool.
pub mod reference_discrepancy;

/// One native side's observation row.
pub mod observation_document;
