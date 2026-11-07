//! What the git adapter asks git with, and the mode it writes a script with.

use crate::domain::git::constants::REV_PARSE_COMMAND;

/// The question that answers where git keeps a path of its own directory, honoring `core.hooksPath` and worktrees.
pub(super) const GIT_PATH: [&str; 2] = [REV_PARSE_COMMAND, "--git-path"];
/// The directory of hooks, in git's own directory.
pub(super) const HOOKS: &str = "hooks";
/// The mode of an executable script.
#[cfg(unix)]
pub(super) const EXECUTABLE: u32 = 0o755;
