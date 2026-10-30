//! The names of the `bases` concept: the marks of a target.

/// Where a fragment of a target starts.
pub(crate) const FRAGMENT: &str = "#";
/// Where a query of a target starts.
pub(crate) const QUERY: &str = "?";
/// What separates the parts of a path of the run.
pub(crate) const SEPARATOR: &str = "/";
/// The name of the file a base that is a directory stands for: only its directory is read.
pub(crate) const IN_DIRECTORY: &str = "_";
