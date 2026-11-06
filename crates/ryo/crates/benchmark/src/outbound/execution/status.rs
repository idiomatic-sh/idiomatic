//! Preserve signal identity without interpreting Windows exit codes as signals.
use ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
/// Decode the OS status; a negative compatibility status is accompanied by an explicit signal.
pub(super) fn decode(
    status: std::process::ExitStatus,
) -> Result<(ProcessExitCode, Option<i32>), Error> {
    if let Some(code) = status.code() {
        return Ok((ProcessExitCode(code), None));
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt as _;
        if let Some(signal) = status.signal() {
            return Ok((ProcessExitCode(-signal), Some(signal)));
        }
    }
    Err(Error::Invalid(
        crate::outbound::execution::constants::CHILD_STATUS_HAS_NEITHER_AN_EXIT.into(),
    ))
}
