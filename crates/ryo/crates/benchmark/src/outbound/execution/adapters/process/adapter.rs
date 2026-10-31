//! Child startup, tool work, serialization and waiting are inside wall time.
use crate::domain::diagnostics::errors::Error;
use crate::domain::measurements::types::{Invocation, ProcessOutput};
use crate::outbound::execution::interface::ExecutionPort;
use std::process::Command;
use std::time::Instant;
#[derive(Debug)]
pub struct ExecutionProcessAdapter;
impl ExecutionPort for ExecutionProcessAdapter {
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        let start = Instant::now();
        let output = Command::new(invocation.executable)
            .args(invocation.arguments)
            .current_dir(invocation.directory)
            .output()?;
        let wall = start.elapsed();
        let status = output
            .status
            .code()
            .ok_or_else(|| Error::Invalid("tool terminated by signal".into()))?;
        Ok(ProcessOutput {
            status,
            stdout: output.stdout,
            stderr: output.stderr,
            wall,
        })
    }
}
