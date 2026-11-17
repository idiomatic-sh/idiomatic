//! The command line vocabulary of the bench operations.
/// What the operation commands take.
pub const USAGE: &str = "usage: bench <languages/<lang>/benchmarks/linters/<linter>>";
/// The refusal of a reference tool whose bytes or version line are not the profile's.
pub const REFERENCE_NOT_VERIFIED: &str =
    "the reference tool is not the pinned one: its bytes or its version line differ";
/// The refusal of a bench whose operation failed.
pub const FAILED: &str = "the bench operation failed; see its output directory";
/// How long one rule's whole worker may run before it is taken to be hung, in seconds; the per-case limit is the harness's
/// `CASE_TIME_LIMIT`.
pub const RULE_TIMEOUT_SECONDS: u64 = 120;
/// The build tool the bench workers are built with, found on the search path.
pub const BUILD_TOOL: &str = "cargo";
/// The variable the build tool that started the collector names its own executable by: the reference is searched first in that
/// executable's directory, where a toolchain ships the linters it carries.
pub const TOOLCHAIN_TOOL: &str = "CARGO";
/// The variable naming the search path the reference tool is found on.
pub const SEARCH_PATH: &str = "PATH";
