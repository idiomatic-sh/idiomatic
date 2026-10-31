//! A deterministic process substitute records the observable result of fixture calls.
use crate::domain::diagnostics::errors::Error;
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
            history: Default::default(),
        }
    }
}
impl ExecutionPort for ExecutionFake {
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        if invocation.executable != Path::new("sh") && invocation.executable != Path::new("fixture")
        {
            return Err(std::io::Error::from(std::io::ErrorKind::NotFound).into());
        }
        self.history
            .lock()
            .expect("fixture history")
            .push(invocation);
        Ok(ProcessOutput {
            status: self.status,
            stdout: b"payload".to_vec(),
            stderr: b"issue".to_vec(),
            wall: Duration::ZERO,
            resources: None,
        })
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
