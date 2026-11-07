//! The words of git's command line the git adapters share.

/// The program.
pub const GIT: &str = "git";
/// The subcommand that answers questions about revisions and paths.
pub const REV_PARSE_COMMAND: &str = "rev-parse";
/// The question that answers where the working tree of the repository starts.
pub const TOP_LEVEL: [&str; 2] = [REV_PARSE_COMMAND, "--show-toplevel"];
/// What git says outside a repository.
pub const NOT_A_REPOSITORY: &str = "not a git repository";
/// The variables a git hook exports for the repository it runs in, which would point git at that one instead of the
/// product's.
pub const HOOK_VARIABLES: [&str; 5] = [
    "GIT_DIR",
    "GIT_INDEX_FILE",
    "GIT_WORK_TREE",
    "GIT_PREFIX",
    "GIT_OBJECT_DIRECTORY",
];
