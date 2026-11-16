//! The files and words the Linux meter reads and writes.
/// The status file of this process, whose `VmHWM` line is the peak resident memory.
pub(super) const STATUS: &str = "/proc/self/status";
/// The file that resets this process's peak resident memory when `5` is written to it.
pub(super) const CLEAR_REFS: &str = "/proc/self/clear_refs";
/// What resets the peak.
pub(super) const RESET_PEAK: &str = "5";
/// The label of the peak resident memory in the status file.
pub(super) const PEAK_LABEL: &str = "VmHWM:";
/// The unit the status file states memory in.
pub(super) const KIBIBYTE: u64 = 1024;
/// The refusal of a status file without a readable peak.
pub(super) const NO_PEAK: &str = "/proc/self/status states no VmHWM in kB";
