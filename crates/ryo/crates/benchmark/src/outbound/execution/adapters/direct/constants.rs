//! Execution observation failures.
pub(super) const SIGNAL: &str = "contestant terminated by signal";
pub(super) const PIPE: &str = "configured child output pipe is missing";
pub(super) const READER: &str = "child output reader panicked";
pub(super) const CPU: &str = "child CPU duration overflow";
