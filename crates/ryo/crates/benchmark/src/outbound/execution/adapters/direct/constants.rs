//! Execution observation failures.
/// Refusal when configured child capture pipes are unavailable.
pub(super) const PIPE: &str = "configured child output pipe is missing";
/// Refusal when a concurrent capture thread panics.
pub(super) const READER: &str = "child output reader panicked";
/// Refusal when summed child CPU durations exceed the representable range.
pub(super) const CPU: &str = "child CPU duration overflow";
