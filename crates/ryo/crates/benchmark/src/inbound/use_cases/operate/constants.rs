//! The command line vocabulary of the bench operations.
/// What the operation commands take.
pub const USAGE: &str = "usage: (verify-inventory | measure-valid | parity) <languages/<lang>/benchmarks/linters/<linter>>";
/// The refusal when the operation failed.
pub const FAILED: &str = "the bench operation failed; see its output directory";
/// How long one rule's verification may take, in seconds.
pub const RULE_TIMEOUT_SECONDS: u64 = 120;
/// The build tool the bench workers are built with, found on the search path.
pub const BUILD_TOOL: &str = "cargo";
/// The variable the build tool that started the collector names its own executable by: the reference is searched first in that
/// executable's directory, where a toolchain ships the linters it carries.
pub const TOOLCHAIN_TOOL: &str = "CARGO";
/// The variable naming the search path the reference tool is found on.
pub const SEARCH_PATH: &str = "PATH";
