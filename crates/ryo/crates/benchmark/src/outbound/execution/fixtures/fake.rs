//! A deterministic process substitute records the observable result of fixture calls.
use crate::domain::diagnostics::errors::DiagnosticsError as Error;
use crate::domain::measurements::types::{CapturedBytes, ProcessExitCode, ResourceObservation};
use crate::domain::measurements::types::{Invocation, ProcessOutput};
use crate::outbound::execution::interface::ExecutionPort;
use std::{path::Path, time::Duration};
#[derive(Clone, Debug)]
/// Deterministic execution substitute for orchestration and contract tests.
pub struct ExecutionFake {
    /// Exit status returned for supported fixture invocations.
    pub status: i32,
    /// Calls actually received, shared with the test observer.
    pub history: std::sync::Arc<std::sync::Mutex<Vec<Invocation>>>,
}
impl Default for ExecutionFake {
    fn default() -> Self {
        Self {
            status: 7,
            history: std::sync::Arc::default(),
        }
    }
}

/// Explicit native-child control exits; the ordinary fixture still uses its configured status.
fn requested_exit(arguments: &[std::ffi::OsString]) -> Option<i32> {
    [
        ("sys.exit(0)", 0),
        ("sys.exit(126)", 126),
        ("sys.exit(127)", 127),
        ("sys.exit(3840)", 3840),
    ]
    .into_iter()
    .find_map(|(ending, status)| {
        arguments
            .iter()
            .any(|argument| argument.to_string_lossy().ends_with(ending))
            .then_some(status)
    })
}

impl ExecutionPort for ExecutionFake {
    fn profile(&self) -> crate::domain::measurements::types::ExecutionProfile {
        crate::domain::measurements::types::ExecutionProfile::Wall
    }
    #[expect(
        clippy::expect_used,
        reason = "a poisoned fixture history lock must fail the test"
    )]
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        if invocation.executable.file_stem().is_some_and(|name| {
            name == "invalid-format" || cfg!(windows) && name == "missing-interpreter"
        }) {
            return Err(std::io::Error::from_raw_os_error(super::driver::image_error()).into());
        }
        let large = invocation
            .arguments
            .iter()
            .any(|argument| argument == super::driver::LARGE_STREAMS);
        if invocation
            .executable
            .file_stem()
            .is_some_and(|name| name == "nonexecutable" || name == "directory")
        {
            return Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied).into());
        }
        if invocation.executable != Path::new(super::driver::python())
            && invocation.executable != Path::new("fixture")
        {
            return Err(std::io::Error::from(std::io::ErrorKind::NotFound).into());
        }
        let signal = invocation
            .arguments
            .iter()
            .any(|argument| cfg!(unix) && argument == super::driver::SIGNAL)
            .then_some(15_i32);
        let status = if let Some(signal) = signal {
            -signal
        } else if large {
            0_i32
        } else {
            requested_exit(&invocation.arguments).unwrap_or(self.status)
        };
        self.history
            .lock()
            .expect("fixture history")
            .push(invocation);
        Ok(ProcessOutput {
            signal,
            status: ProcessExitCode(status),
            stdout: CapturedBytes(if signal.is_some() {
                Vec::new()
            } else if large {
                vec![0; 131_072]
            } else {
                b"payload".to_vec()
            }),
            stderr: CapturedBytes(if signal.is_some() {
                Vec::new()
            } else if large {
                vec![0; 131_072]
            } else {
                b"issue".to_vec()
            }),
            wall: Duration::ZERO,
            resources: ResourceObservation::WallOnly,
        })
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
