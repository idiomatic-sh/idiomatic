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
    fn rule_configuration(&self) -> crate::domain::suite::types::RuleConfigurationSupport {
        crate::domain::suite::types::RuleConfigurationSupport::Unsupported
    }
    fn prepare(&self, request: &RunRequest) -> Result<Invocation, Error> {
        Ok(Invocation {
            executable: PathBuf::from("fixture"),
            arguments: self.arguments.clone(),
            environment: std::collections::BTreeMap::default(),
            directory: request.root.clone(),
        })
    }
    fn decode(&self, request: &RunRequest, output: ProcessOutput) -> Result<RunReport, Error> {
        if output.signal.is_some() || !(0_i32..=1_i32).contains(&output.status.0) {
            return Err(Error::Invalid("operational failure".into()));
        }
        Ok(RunReport {
            execution_metadata: crate::domain::diagnostics::types::ExecutionMetadata::Complete,
            project_context: None,
            files: request.files.clone(),
            skipped: Vec::new(),
            coverage: Some(crate::domain::diagnostics::types::RuleCoverage {
                selected: request.files.len(),
                evaluated: request.files.len(),
                skipped: 0,
                unknown_or_unread: 0,
            }),
            diagnostics: self.diagnostics.clone(),
        })
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
