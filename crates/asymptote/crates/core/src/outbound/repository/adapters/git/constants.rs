//! What the git adapter asks git with.

use crate::domain::git::constants::REV_PARSE_COMMAND;

/// The separator of the sections `--format=%x1e` writes.
pub(super) const RECORD: char = '\u{1e}';
/// The revision the working tree stands on.
pub(super) const HEAD: &str = crate::domain::reports::constants::HEAD;
/// What peels a revision to its commit, after it.
pub(super) const PEEL: &str = "^{commit}";
/// The commit a revision names, quietly.
pub(super) const REV_PARSE: [&str; 4] =
    [REV_PARSE_COMMAND, "--verify", "--quiet", "--end-of-options"];
/// The merge base of two commits.
pub(super) const MERGE_BASE: &str = "merge-base";
/// The renames to the index and working tree: git's rename detection at 50 % similarity.
pub(super) const DIFF: [&str; 6] = [
    "diff",
    "--relative",
    "--name-status",
    "--find-renames=50%",
    "-z",
    "--end-of-options",
];
/// The renames of every first-parent commit, oldest first.
pub(super) const LOG_RENAMES: [&str; 9] = [
    "log",
    "--relative",
    "--first-parent",
    "--reverse",
    "--format=format:%x1e%H%x00",
    "--name-status",
    "-z",
    "--find-renames=50%",
    "--end-of-options",
];
/// The files each first-parent commit added, oldest first.
pub(super) const LOG_ADDED: [&str; 8] = [
    "log",
    "--relative",
    "--first-parent",
    "--reverse",
    "--format=format:%x1e%H%x00%P%x00",
    "--name-only",
    "--diff-filter=A",
    "-z",
];
/// What separates options from the revision and the paths that follow it.
pub(super) const END: &str = "--end-of-options";
/// What separates the revision from the paths.
pub(super) const PATHS: &str = "--";
/// The files of a tree beneath a path.
pub(super) const LS_TREE: [&str; 4] = ["ls-tree", "-r", "-z", "--name-only"];
/// A file's text at a revision.
pub(super) const SHOW: &str = "show";
/// The files of the working tree, tracked and untracked but not ignored, from the repository's top.
pub(super) const LS_FILES: [&str; 5] = [
    "ls-files",
    "-z",
    "--cached",
    "--others",
    "--exclude-standard",
];
/// What separates a path's segments as the repository writes them.
pub(super) const SEPARATOR: &str = "/";
/// Which of the paths that follow are ignored, each answered on its own line, ignored or not.
pub(super) const CHECK_IGNORE: [&str; 4] = ["check-ignore", "--verbose", "--non-matching", "--"];
/// The status `CHECK_IGNORE` exits with when it answers that no path is ignored.
pub(super) const NONE_IGNORED: i32 = 1;
/// What begins the answer of `CHECK_IGNORE` for a path no pattern matches.
pub(super) const UNMATCHED: &str = "::";
/// What separates the pattern `CHECK_IGNORE` answers from the path.
pub(super) const BEFORE_PATH: char = '\t';
/// What separates the source, line and pattern `CHECK_IGNORE` answers.
pub(super) const FIELD: char = ':';
/// What begins a pattern that un-ignores what it matches.
pub(super) const NEGATED: char = '!';
/// The ignored files beneath the paths that follow.
pub(super) const IGNORED: [&str; 6] = [
    "ls-files",
    "-z",
    "--others",
    "--ignored",
    "--exclude-standard",
    "--",
];
/// The untracked files that are not ignored.
pub(super) const UNTRACKED: [&str; 4] = ["ls-files", "-z", "--others", "--exclude-standard"];
/// Where the repository's index file is.
pub(super) const INDEX: [&str; 4] = [
    REV_PARSE_COMMAND,
    "--path-format=absolute",
    "--git-path",
    "index",
];
/// Every untracked file added to an index as intent-to-add, so a diff reads it.
pub(super) const INTENT_TO_ADD: [&str; 4] = ["add", "--intent-to-add", "--", "."];
/// Every pathspec read literally.
pub(super) const LITERAL: &str = "--literal-pathspecs";
/// The variable that names the index git reads.
pub(super) const INDEX_FILE: &str = "GIT_INDEX_FILE";
/// What joins a range's two ends.
pub(super) const RANGE: &str = "..";
/// What joins a revision and a path below the current directory, `<rev>:./<path>`.
pub(super) const AT_PATH: &str = ":./";
