//! The command line vocabulary of the inventory verification.
/// The command word that selects the verification.
pub const COMMAND: &str = "verify-inventory";
/// What the command takes.
pub const USAGE: &str = "usage: verify-inventory <registration.json> <output> <timeout-seconds>";
/// The refusal when the timeout argument is not text.
pub const TIMEOUT_NOT_UTF8: &str = "timeout is not UTF-8";
/// The refusal when the timeout is not a positive finite number.
pub const TIMEOUT_INVALID: &str = "timeout must be finite and positive";
/// The refusal when any rule failed.
pub const FAILED: &str = "inventory contains failed rules; see summary.json";
