//! The words the path adapter writes.

/// The most packages the adapter holds in memory; past it the one held longest is let go.
pub(super) const HELD_PACKAGES: usize = 65_536;

/// Where the packages the run made and holds in memory are read: one directory each, numbered, on no disk.
pub(super) const HELD: &str = "/tastemaker-held/";
/// Why a file a held package does not hold cannot be read.
pub(super) const MISSING: &str = "the package the run made holds no such file";
/// A git source, as a refusal names it.
pub(super) const GIT: &str = "git";
/// The registry, as a refusal names it.
pub(super) const REGISTRY: &str = "the registry";
/// What separates the segments of a path written in a manifest.
pub(super) const SEPARATOR: char = '/';
/// The same, as text to join segments with.
pub(super) const SEPARATOR_TEXT: &str = "/";
/// The segment that names the directory it is in.
pub(super) const CURRENT: &str = crate::domain::paths::constants::CURRENT;
/// The segment that names the directory above.
pub(super) const PARENT: &str = crate::domain::paths::constants::PARENT;
/// Why a name in a package is refused when it is not UTF-8: a definition's path is text.
pub(super) const NOT_UTF8: &str = crate::domain::paths::constants::NOT_UTF8;
