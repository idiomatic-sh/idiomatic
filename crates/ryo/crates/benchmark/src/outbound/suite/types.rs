//! Per-operation artifact identities, independent of adapter configuration.
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
/// A directory the port made and keeps until it is discarded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScratchDirectory(pub std::path::PathBuf);

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
    /// The context sealed for the case.
    pub context: MaterializedContext,
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
