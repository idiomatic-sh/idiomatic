//! Collector operational refusals.
/// Refusal when collector bytes no longer match the pinned identity.
pub(super) const DIGEST: &str = "collector executable digest changed";
/// Refusal when the collector request pipe is unavailable.
pub(super) const PIPE: &str = "collector input pipe missing";
/// Refusal when the request writer thread panics.
pub(super) const WRITER: &str = "collector input writer panicked";
/// Prefix identifying collector process failure rather than contestant exit.
pub(super) const FAILED: &str = "collector process failed";
