//! The names of the `scenario_files` concept: the files a scenario is written in.

/// The file a scenario's project is placed by.
pub const WORKSPACE: &str = crate::domain::definitions::constants::WORKSPACE_FILE;

/// Where a scenario's scratch repository says its branch, so a tool that reads git finds one.
pub const GIT_HEAD: &str = ".git/HEAD";

/// What the scratch repository's branch file holds.
pub const GIT_HEAD_TEXT: &str = "ref: refs/heads/main\n";

/// The glob of every path.
pub const EVERY_PATH: &str = crate::domain::selectors::constants::EVERY_PATH;

/// How a directory that is the root itself is written.
pub const HERE: &str = crate::domain::paths::constants::CURRENT;

/// The rule a finding has none of.
pub const NO_RULE: &str = "";

/// The check a scenario leaves out of its comparison: it proves its own release.
pub const OUTDATED_CHECK: &str = "#outdated";

/// What a passing scenario's `then.findings` is.
pub const EMPTY: &str = "empty";

/// What a failing scenario's `then.findings` is.
pub const NOT_EMPTY: &str = "not empty";

/// What a scenario that must have no finding is called, in a message.
pub const PASSING: &str = "passing";

/// What a scenario that must have findings is called, in a message.
pub const FAILING: &str = "failing";
