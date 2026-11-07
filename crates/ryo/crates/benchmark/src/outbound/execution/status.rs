//! Preserve signal identity without interpreting Windows exit codes as signals.
use crate::outbound::execution::constants;
use ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
/// Decode the OS status; a negative compatibility status is accompanied by an explicit signal.
pub(super) fn decode(
    status: std::process::ExitStatus,
) -> Result<(ProcessExitCode, Option<i32>), Error> {
    if let Some(code) = status.code() {
        return Ok((ProcessExitCode(code), None));
    }
    if let Some(Signal(signal)) = terminating_signal(status) {
        return Ok((ProcessExitCode(-signal), Some(signal)));
    }
    Err(Error::Invalid(
        constants::CHILD_STATUS_HAS_NEITHER_AN_EXIT.into(),
    ))
}

/// A signal number the platform reports for a child it ended.
struct Signal(i32);

/// The signal that ended the child, where the platform reports one.
#[cfg(unix)]
fn terminating_signal(status: std::process::ExitStatus) -> Option<Signal> {
    use std::os::unix::process::ExitStatusExt as _;
    status.signal().map(Signal)
}

/// Platforms without signals never report one.
#[cfg(not(unix))]
fn terminating_signal(_status: std::process::ExitStatus) -> Option<Signal> {
    None
}
