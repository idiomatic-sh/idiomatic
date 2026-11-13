//! The command line vocabulary of the bench operations.
/// What the operation commands take.
pub const USAGE: &str = "usage: (verify-inventory | measure-valid | parity) <languages/<lang>/benchmarks/linters/<linter>>";
/// The refusal when the operation failed.
pub const FAILED: &str = "the bench operation failed; see its output directory";
/// How long one rule's verification may take, in seconds.
pub const RULE_TIMEOUT_SECONDS: u64 = 120;
