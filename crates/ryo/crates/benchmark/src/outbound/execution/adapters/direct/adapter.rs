//! Direct Linux wait4 observes the exact child started by std::process::Command.
#[cfg(target_os = "linux")]
use super::{constants, schemas};
use crate::domain::measurements::types::{CapturedBytes, ProcessExitCode, ResourceObservation};
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    measurements::types::{Invocation, ProcessOutput},
};
use crate::outbound::execution::interface::ExecutionPort;

#[derive(Debug)]
pub(crate) struct ExecutionDirectAdapter;
impl ExecutionPort for ExecutionDirectAdapter {
    fn profile(&self) -> crate::domain::measurements::types::ExecutionProfile {
        crate::domain::measurements::types::ExecutionProfile::Direct
    }
    #[cfg(target_os = "linux")]
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        use std::io::Read;
        use std::process::{Command, Stdio};
        use std::time::Instant;
        use wait4::Wait4;

        let start = Instant::now();
        let mut child = Command::new(invocation.executable)
            .args(invocation.arguments)
            .envs(invocation.environment)
            .current_dir(invocation.directory)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Invalid(constants::PIPE.into()))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| Error::Invalid(constants::PIPE.into()))?;
        // Both pipes drain concurrently while wait4 owns and reaps exactly this child.
        // wait4 consumes Child, retries EINTR, and returns per-child CPU and RSS.
        let (usage, stdout, stderr) = std::thread::scope(|scope| -> Result<_, Error> {
            let output = scope.spawn(move || {
                let mut bytes = Vec::new();
                stdout.read_to_end(&mut bytes).map(|_| bytes)
            });
            let errors = scope.spawn(move || {
                let mut bytes = Vec::new();
                stderr.read_to_end(&mut bytes).map(|_| bytes)
            });
            let usage = child.wait4()?;
            let stdout = output
                .join()
                .map_err(|_| Error::Invalid(constants::READER.into()))??;
            let stderr = errors
                .join()
                .map_err(|_| Error::Invalid(constants::READER.into()))??;
            Ok((usage, stdout, stderr))
        })?;
        let wall = start.elapsed();
        let status = usage
            .status
            .code()
            .ok_or_else(|| Error::Invalid(constants::SIGNAL.into()))?;
        Ok(ProcessOutput {
            status: ProcessExitCode(status),
            stdout: CapturedBytes(stdout),
            stderr: CapturedBytes(stderr),
            wall,
            resources: ResourceObservation::Collected(schemas::decode(usage.rusage)?),
        })
    }

    #[cfg(not(target_os = "linux"))]
    fn run(&self, _: Invocation) -> Result<ProcessOutput, Error> {
        // Refuse platforms where the dependency can fall back to cumulative child usage.
        Err(std::io::Error::from(std::io::ErrorKind::Unsupported).into())
    }
}
