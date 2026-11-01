//! Child startup, tool work, serialization and waiting are inside wall time.
use crate::domain::diagnostics::errors::DiagnosticsError as Error;
use crate::domain::measurements::types::{CapturedBytes, ResourceObservation};
use crate::domain::measurements::types::{Invocation, ProcessOutput};
use crate::outbound::execution::interface::ExecutionPort;
use std::process::Command;
use std::time::Instant;
#[derive(Debug)]
/// Executes a child with wall time and captured output, without resource claims.
pub(crate) struct ExecutionProcessAdapter;
impl ExecutionPort for ExecutionProcessAdapter {
    fn profile(&self) -> crate::domain::measurements::types::ExecutionProfile {
        crate::domain::measurements::types::ExecutionProfile::Wall
    }
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        let start = Instant::now();
        let output = Command::new(invocation.executable)
            .args(invocation.arguments)
            .envs(invocation.environment)
            .current_dir(invocation.directory)
            .output()?;
        let wall = start.elapsed();
        let (status, signal) = crate::outbound::execution::status::decode(output.status)?;
        Ok(ProcessOutput {
            signal,
            status,
            stdout: CapturedBytes(output.stdout),
            stderr: CapturedBytes(output.stderr),
            wall,
            resources: ResourceObservation::WallOnly,
        })
    }
}
