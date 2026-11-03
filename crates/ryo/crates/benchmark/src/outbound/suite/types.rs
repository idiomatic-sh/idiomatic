//! Per-operation artifact identities, independent of adapter configuration.
/// A source, destination or executable artifact selected for this operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactPath<'a>(pub &'a std::path::Path);
/// Exact artifact paths whose names and bytes participate in one fingerprint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactSet<'a>(pub &'a [std::path::PathBuf]);

/// Reference evidence wire values owned by this boundary.
pub mod discrepancy;

/// Byte-exact expected process failure artifacts and templates.
pub mod process;
