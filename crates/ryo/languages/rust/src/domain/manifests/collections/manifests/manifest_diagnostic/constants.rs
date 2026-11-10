//! Stable labels for the diagnostics of manifests Cargo's reading refused.

/// Where a diagnostic with no finer place is: the origin of the file.
pub(super) const FILE_ORIGIN: &str = "file-origin";

/// The stage of a manifest the reader could not read.
pub(super) const PARSE: &str = "parse";

/// The stage of a manifest read whose workspace is not consistent.
pub(super) const VALIDATION: &str = "validation";

/// The schema of a `Cargo.toml`.
pub(super) const CARGO_MANIFEST: &str = "cargo-manifest";

/// The category of a manifest the reader refused.
pub(super) const UNREADABLE_MANIFEST: &str = "unreadable-manifest";

/// The category of a manifest whose workspace does not hold it, or that takes a value from a workspace it is not in.
pub(super) const INCONSISTENT_WORKSPACE: &str = "inconsistent-workspace";
