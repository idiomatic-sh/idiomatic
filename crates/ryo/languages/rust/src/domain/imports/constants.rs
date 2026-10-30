//! The names of the `imports` concept: the text a `use` is written with.

/// The glob of a `use`.
pub(crate) const GLOB: &str = crate::domain::module_tree::constants::GLOB;
/// The name a rename to `_` binds.
pub(crate) const UNDERSCORE: &str = "_";
/// What an absolute path starts with.
pub(crate) const ROOT: &str = "::";
/// The path segment that names the tree above it.
pub(crate) const SELF_SEGMENT: &str = "self";
