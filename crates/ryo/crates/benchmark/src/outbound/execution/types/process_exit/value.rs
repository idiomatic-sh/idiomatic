//! Preserve signal identity without interpreting Windows exit codes as signals.
use crate::outbound::execution::errors::ExecutionError;
use ryo_benchmark_contracts::domain::measurements::types::{
    ProcessExitCode, SignalNumber, TerminationSignal,
};

/// The decoded exit of one child: its code and how the operating system ended it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProcessExit {
    /// Exit code; a negative compatibility status accompanies an explicit signal.
    pub(crate) code: ProcessExitCode,
    /// The signal that ended the child, where the platform reports one.
    pub(crate) termination: TerminationSignal,
}

/// Decode the OS status; a negative compatibility status is accompanied by an explicit signal.
pub(crate) fn decode(status: std::process::ExitStatus) -> Result<ProcessExit, ExecutionError> {
    if let Some(code) = status.code() {
        return Ok(ProcessExit {
            code: ProcessExitCode(code),
            termination: TerminationSignal::Exited,
        });
    }
    if let Some(SignalNumber(signal)) = terminating_signal(status) {
        return Ok(ProcessExit {
            code: ProcessExitCode(-signal),
            termination: TerminationSignal::Signalled(
                ryo_benchmark_contracts::domain::measurements::types::SignalNumber(signal),
            ),
        });
    }
    Err(ExecutionError::UnobservedExit)
}

/// The signal that ended the child, where the platform reports one.
#[cfg(unix)]
fn terminating_signal(status: std::process::ExitStatus) -> Option<SignalNumber> {
    use std::os::unix::process::ExitStatusExt as _;
    status.signal().map(SignalNumber)
}

/// Platforms without signals never report one.
#[cfg(not(unix))]
fn terminating_signal(_status: std::process::ExitStatus) -> Option<SignalNumber> {
    None
}
