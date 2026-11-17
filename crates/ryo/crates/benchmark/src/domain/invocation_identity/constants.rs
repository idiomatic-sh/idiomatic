//! The placeholders a normalized invocation uses.
pub(super) const STATE: &str = "$STATE";
pub(super) const ROOT: &str = "$ROOT";
pub(super) const NORMALIZED_SEPARATOR: &str = "/";
pub(super) const BACKSLASH: char = '\\';
/// The worker count every identity spells, whatever the run takes: the workers a reference runs on are a measurement setting
/// no finding depends on, and the recorded identities were made at one worker.
pub(super) const IDENTITY_WORKERS: &str = "1";
