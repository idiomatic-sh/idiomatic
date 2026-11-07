//! The names the memory adapter looks for and skips.

/// The most scratch directories the adapter holds at once; past it a new one is refused.
pub(super) const TREES: usize = 16_384;
/// Why a scratch directory is refused when the adapter holds as many as it may.
pub(super) const TOO_MANY: &str = "the adapter holds as many scratch directories as it may";

/// A workspace's file.
pub(super) const WORKSPACE_FILE: &str = crate::domain::definitions::constants::WORKSPACE_FILE;
/// What marks a repository's root, and the directory every walk skips.
pub(super) const GIT: &str = crate::domain::definitions::constants::GIT;
/// The ignore file git reads in each directory.
pub(super) const GIT_IGNORE: &str = crate::domain::definitions::constants::GITIGNORE;
/// The ignore file `ignore` reads in each directory, which a `.gitignore` of the same directory cannot override.
pub(super) const IGNORE: &str = ".ignore";
/// Who declares the directories every walk skips, as `explain` says it.
pub(super) const TASTEMAKER: &str = crate::domain::findings::constants::TASTEMAKER;
/// The code Linux, macOS and Windows each give a file that is not there.
pub(super) const MISSING_FILE: i32 = 2;
/// Why a project made over a directory that is there is refused.
pub(super) const ALREADY_THERE: &str = "it is already there";
