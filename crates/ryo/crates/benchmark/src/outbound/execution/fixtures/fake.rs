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
impl ExecutionPort for ExecutionFake {
    fn profile(&self) -> crate::domain::measurements::types::ExecutionProfile {
        crate::domain::measurements::types::ExecutionProfile::Wall
    }
    #[expect(
        clippy::expect_used,
        reason = "a poisoned fixture history lock must fail the test"
    )]
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        if invocation
            .executable
            .file_name()
            .is_some_and(|name| name == "invalid-format")
        {
            return Err(std::io::Error::from_raw_os_error(8).into());
        }
        if invocation
            .arguments
            .iter()
            .any(|argument| argument == "kill -TERM $$")
        {
            return Err(Error::Invalid("fixture child terminated by signal".into()));
        }
        let large = invocation
            .arguments
            .iter()
            .any(|argument| argument.to_string_lossy().contains("/dev/zero"));
        if invocation
            .executable
            .file_name()
            .is_some_and(|name| name == "nonexecutable" || name == "directory")
        {
            return Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied).into());
        }
        if invocation.executable != Path::new("sh") && invocation.executable != Path::new("fixture")
        {
            return Err(std::io::Error::from(std::io::ErrorKind::NotFound).into());
        }
        let status = if large {
            0_i32
        } else if invocation
            .arguments
            .iter()
            .any(|argument| argument.to_string_lossy().ends_with("exit 127"))
        {
            127_i32
        } else if invocation
            .arguments
            .iter()
            .any(|argument| argument.to_string_lossy().ends_with("exit 126"))
        {
            126_i32
        } else {
            self.status
        };
        self.history
            .lock()
            .expect("fixture history")
            .push(invocation);
        Ok(ProcessOutput {
            status: ProcessExitCode(status),
            stdout: CapturedBytes(if large {
                vec![0; 131_072]
            } else {
                b"payload".to_vec()
            }),
            stderr: CapturedBytes(if large {
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
