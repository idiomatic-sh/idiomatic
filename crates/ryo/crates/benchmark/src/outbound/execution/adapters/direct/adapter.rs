//! Direct Linux `wait4` observes the exact child started by `std::process::Command`.
#[cfg(target_os = "linux")]
use super::{constants, schemas};
use crate::outbound::execution::interface::ExecutionPort;
#[cfg(target_os = "linux")]
use crate::outbound::execution::types::process_exit::value::decode;
#[cfg(target_os = "linux")]
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::measurements::types::ResourceObservation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

#[derive(Debug)]
/// Collects resources for one directly owned child on Linux.
pub(crate) struct ExecutionDirectAdapter;
impl ExecutionPort for ExecutionDirectAdapter {
    fn profile(&self) -> ryo_benchmark_contracts::domain::measurements::types::ExecutionProfile {
        ryo_benchmark_contracts::domain::measurements::types::ExecutionProfile::Direct
    }
    #[cfg(target_os = "linux")]
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        use std::io::Read as _;
        use std::process::{Command, Stdio};
        use std::time::Instant;
        use wait4::Wait4 as _;

        let start = Instant::now();
        let mut child = Command::new(invocation.executable)
            .args(invocation.arguments)
            .envs(invocation.environment)
            .current_dir(invocation.directory)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut stdout_pipe = child
            .stdout
            .take()
            .ok_or_else(|| Error::Invalid(constants::PIPE.into()))?;
        let mut stderr_pipe = child
            .stderr
            .take()
            .ok_or_else(|| Error::Invalid(constants::PIPE.into()))?;
        // Both pipes drain concurrently while wait4 owns and reaps exactly this child.
        // wait4 consumes Child, retries EINTR, and returns per-child CPU and RSS.
        let (usage, stdout, stderr) = std::thread::scope(|scope| -> Result<_, Error> {
            let output = scope.spawn(move || {
                let mut bytes = Vec::new();
                stdout_pipe.read_to_end(&mut bytes).map(|_| bytes)
            });
            let errors = scope.spawn(move || {
                let mut bytes = Vec::new();
                stderr_pipe.read_to_end(&mut bytes).map(|_| bytes)
            });
            let usage = child.wait4()?;
            let stdout = output
                .join()
                .map_err(|_reader_panicked| Error::Invalid(constants::READER.into()))??;
            let stderr = errors
                .join()
                .map_err(|_reader_panicked| Error::Invalid(constants::READER.into()))??;
            Ok((usage, stdout, stderr))
        })?;
        let wall = start.elapsed();
        let exit = decode(usage.status).map_err(|error| Error::Invalid(error.to_string()))?;
        Ok(ProcessOutput {
            signal: exit.termination,
            status: exit.code,
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
