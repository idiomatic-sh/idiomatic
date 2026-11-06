//! Stable labels for the diagnostics of manifests the reader refused.

/// Where a diagnostic with no finer place is: the origin of the file.
pub(super) const FILE_ORIGIN: &str = "file-origin";

/// The stage of a manifest the reader could not read.
pub(super) const PARSE: &str = "parse";

/// The stage of a manifest read whose references do not hold.
pub(super) const VALIDATION: &str = "validation";

/// The category of a manifest the reader refused.
pub(super) const UNREADABLE_MANIFEST: &str = "unreadable-manifest";

/// The category of a `catalog:` requirement no catalog has an entry for.
pub(super) const UNRESOLVED_CATALOG_ENTRY: &str = "unresolved-catalog-entry";
