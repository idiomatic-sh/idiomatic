//! What the git adapter asks git with, and the child it writes an executable script with.

use asymptote_values::domain::git::constants::REV_PARSE_COMMAND;

/// The question that answers where git keeps a path of its own directory, honoring `core.hooksPath` and worktrees.
pub(super) const GIT_PATH: [&str; 2] = [REV_PARSE_COMMAND, "--git-path"];
/// The directory of hooks, in git's own directory.
pub(super) const HOOKS: &str = "hooks";
/// The shell a child writes a hook script with.
#[cfg(unix)]
pub(super) const SHELL: &str = "/bin/sh";
/// What that child runs: it writes its standard input to the script (`$1`) and makes it executable (0755). A child writes it,
/// so no descriptor this multi-threaded process holds for writing can leak into a child another thread forks, and the hook's
/// first launch never fails with "Text file busy" (ETXTBSY).
#[cfg(unix)]
pub(super) const WRITE_EXECUTABLE: [&str; 3] = ["-c", "cat > \"$1\" && chmod 755 \"$1\"", "sh"];
