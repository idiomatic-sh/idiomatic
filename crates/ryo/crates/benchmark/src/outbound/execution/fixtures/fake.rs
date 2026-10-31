//! A deterministic process substitute records the observable result of fixture calls.
use crate::domain::diagnostics::errors::Error;
use crate::domain::measurements::types::{Invocation, ProcessOutput};
use crate::outbound::execution::interface::ExecutionPort;
use std::{path::Path, time::Duration};
#[derive(Debug)]
pub struct ExecutionFake {
    pub status: i32,
}
impl Default for ExecutionFake {
    fn default() -> Self {
        Self { status: 7 }
    }
}
impl ExecutionPort for ExecutionFake {
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        if invocation.executable != Path::new("sh") && invocation.executable != Path::new("fixture")
        {
            return Err(std::io::Error::from(std::io::ErrorKind::NotFound).into());
        }
        Ok(ProcessOutput {
            status: self.status,
            stdout: b"payload".to_vec(),
            stderr: b"issue".to_vec(),
            wall: Duration::ZERO,
        })
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
