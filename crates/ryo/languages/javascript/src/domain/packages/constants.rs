//! The filenames and separators in repository package paths.

/// The file name of an npm package manifest.
pub(crate) const PACKAGE: &str = "package.json";
/// What separates the parts of a repository path.
pub(crate) const SEPARATOR: char = '/';
/// The part an empty path or a doubled separator gives, which moves nowhere.
pub(crate) const EMPTY: &str = "";
/// The part that stays in the current directory.
pub(crate) const CURRENT: &str = ".";
/// The part that goes up one directory.
pub(crate) const PARENT: &str = "..";
