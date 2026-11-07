//! Child startup, tool work, serialization and waiting are inside wall time.
use crate::outbound::execution::adapters::process::constants;
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::execution::types::ExitObservationMode;
use crate::outbound::execution::types::process_exit::value::decode;
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::measurements::types::ResourceObservation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::{
    io::{Read as _, Seek as _},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// Native process execution with an optional per-invocation deadline.
#[derive(Debug)]
pub(crate) struct ExecutionProcessAdapter {
    /// No deadline for existing measurements; inventories explicitly bound each invocation.
    pub(crate) timeout: Option<Duration>,
}
impl ExecutionPort for ExecutionProcessAdapter {
    fn exit_observation(&self) -> ExitObservationMode {
        if self.timeout.is_none() {
            ExitObservationMode::Direct
        } else {
            ExitObservationMode::Mediated
        }
    }

    fn profile(&self) -> ryo_benchmark_contracts::domain::measurements::types::ExecutionProfile {
        ryo_benchmark_contracts::domain::measurements::types::ExecutionProfile::Wall
    }
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        let start = Instant::now();
        let mut command = Command::new(invocation.executable);
        let _command = command
            .args(invocation.arguments)
            .envs(invocation.environment)
            .current_dir(invocation.directory);
        let (status, signal, stdout, stderr) = if let Some(timeout) = self.timeout {
            // Files avoid pipe-reader threads that could remain blocked after a timed-out descendant.
            let mut stdout = tempfile::tempfile()?;
            let mut stderr = tempfile::tempfile()?;
            let _command = command
                .stdout(Stdio::from(stdout.try_clone()?))
                .stderr(Stdio::from(stderr.try_clone()?));
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt as _;
                let _command = command.process_group(0);
            }
            let mut child = command.spawn()?;
            let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
                Error::Invalid(constants::PROCESS_DEADLINE_IS_OUT_OF_RANGE.into())
            })?;
            let mut timed_out = false;
            let status = loop {
                if let Some(status) = child.try_wait()? {
                    break status;
                }
                if Instant::now() >= deadline {
                    timed_out = true;
                    #[cfg(unix)]
                    {
                        let _result = Command::new(constants::KILL)
                            .args([
                                constants::KILL_2,
                                constants::TEXT,
                                &format!("-{}", child.id()),
                            ])
                            .status();
                    }
                    #[cfg(windows)]
                    {
                        let _result = Command::new(constants::TASKKILL)
                            .args([
                                constants::PID,
                                &child.id().to_string(),
                                constants::T,
                                constants::F,
                            ])
                            .status();
                    }
                    let _result = child.kill();
                    break child.wait()?;
                }
                std::thread::sleep(Duration::from_millis(10));
            };
            stdout.rewind()?;
            stderr.rewind()?;
            let mut out = Vec::new();
            let mut err = Vec::new();
            let _stdout_bytes = stdout.read_to_end(&mut out)?;
            let _stderr_bytes = stderr.read_to_end(&mut err)?;
            if timed_out {
                err.extend_from_slice(
                    format!(
                        "\nverifier timeout after {} seconds\n",
                        timeout.as_secs_f64()
                    )
                    .as_bytes(),
                );
                (
                    ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode(124),
                    ryo_benchmark_contracts::domain::measurements::types::TerminationSignal::Exited,
                    out,
                    err,
                )
            } else {
                let exit = decode(status).map_err(|error| Error::Invalid(error.to_string()))?;
                (exit.code, exit.termination, out, err)
            }
        } else {
            let output = command.output()?;
            let exit = decode(output.status).map_err(|error| Error::Invalid(error.to_string()))?;
            (
                exit.code,
                exit.termination,
                output.stdout,
                output.stderr,
            )
        };
        Ok(ProcessOutput {
            signal,
            status,
            stdout: CapturedBytes(stdout),
            stderr: CapturedBytes(stderr),
            wall: start.elapsed(),
            resources: ResourceObservation::WallOnly,
        })
    }
}
