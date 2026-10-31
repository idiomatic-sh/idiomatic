//! Deterministic linter output for orchestration tests.
use crate::domain::measurements::types::{Invocation, ProcessOutput};
use crate::domain::{
    diagnostics::{errors::Error, types::RunReport},
    suite::types::RunRequest,
};
use crate::outbound::linter::interface::LinterPort;
use std::path::PathBuf;
#[derive(Debug, Default)]
pub struct LinterFake;
impl LinterPort for LinterFake {
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        Ok(Invocation {
            executable: PathBuf::from("fixture"),
            arguments: Vec::new(),
            directory: request.root.clone(),
        })
    }
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        if output.status > 1 {
            return Err(Error::Invalid("operational failure".into()));
        }
        Ok(RunReport {
            files: request.files.clone(),
            diagnostics: Vec::new(),
        })
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
