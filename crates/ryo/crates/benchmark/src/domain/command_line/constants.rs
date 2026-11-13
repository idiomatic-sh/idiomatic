//! The worker and measure commands every linter's benchmark executable accepts, and their refusals.
/// The timed worker command: no tracing.
pub(super) const RUN_RYO: &str = "run-ryo";
/// The profiling worker command: a Chrome trace.
pub(super) const PROFILE_RYO: &str = "profile-ryo";
/// The diagnostic worker command: flushed JSONL events.
pub(super) const DIAGNOSTIC_RYO: &str = "diagnostic-ryo";
/// Refusal of a worker command line without a request.
pub(super) const MISSING_WORKER_REQUEST: &str = "missing worker request";
/// Accounted full-input measurement command.
pub(super) const MEASURE: &str = "measure";
/// Accounted measurement command with explicitly audited invalid-source exclusions.
pub(super) const MEASURE_ACCOUNTED_VALID: &str = "measure-valid";
