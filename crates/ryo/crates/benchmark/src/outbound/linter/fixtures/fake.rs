//! Deterministic linter output for orchestration tests.
use crate::domain::measurements::types::{Invocation, ProcessOutput};
use crate::domain::{
    diagnostics::{
        errors::DiagnosticsError as Error,
        types::{BenchmarkFinding, RunReport},
    },
    suite::types::RunRequest,
};
use crate::outbound::linter::interface::LinterPort;
use std::path::PathBuf;
#[derive(Debug, Default)]
/// Deterministic linter substitute for orchestration and contract tests.
pub struct LinterFake {
    /// Canonical findings returned by successful fixture decoding.
    pub diagnostics: Vec<BenchmarkFinding>,
    /// Labels carried through execution for call-order assertions.
    pub arguments: Vec<std::ffi::OsString>,
}
impl LinterPort for LinterFake {
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        Ok(Invocation {
            executable: PathBuf::from("fixture"),
            arguments: self.arguments.clone(),
            environment: std::collections::BTreeMap::default(),
            directory: request.root.clone(),
        })
    }
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        if output.status.0 > 1 {
            return Err(Error::Invalid("operational failure".into()));
        }
        Ok(RunReport {
            files: request.files.clone(),
            diagnostics: self.diagnostics.clone(),
        })
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
