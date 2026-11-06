//! The text this part uses.
/// Text of the process deadline is out of range in this part.
pub(super) const PROCESS_DEADLINE_IS_OUT_OF_RANGE: &str = "process deadline is out of range";
/// Text of the kill in this part.
pub(super) const KILL: &str = "kill";
/// Text of the kill 2 in this part.
pub(super) const KILL_2: &str = "-KILL";
/// Text of the text in this part.
pub(super) const TEXT: &str = "--";
/// Text of the taskkill in this part.
#[cfg(windows)]
pub(super) const TASKKILL: &str = "taskkill";
/// Text of the pid in this part.
#[cfg(windows)]
pub(super) const PID: &str = "/PID";
/// Text of the t in this part.
#[cfg(windows)]
pub(super) const T: &str = "/T";
/// Text of the f in this part.
#[cfg(windows)]
pub(super) const F: &str = "/F";
