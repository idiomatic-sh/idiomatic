//! The names the disk adapter looks for and skips.

/// A workspace's file.
pub(super) const WORKSPACE_FILE: &str = crate::domain::definitions::constants::WORKSPACE_FILE;
/// What marks a repository's root: a directory, or a file in a linked worktree.
pub(super) const GIT: &str = crate::domain::definitions::constants::GIT;
/// Who declares the directories every walk skips, as `explain` says it.
pub(super) const TASTEMAKER: &str = crate::domain::findings::constants::TASTEMAKER;
/// Why a path in the tree is refused when its name is not UTF-8: every path a rule reads and a finding names is text.
pub(super) const NOT_UTF8: &str = crate::domain::paths::constants::NOT_UTF8;
