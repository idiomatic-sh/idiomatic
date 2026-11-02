//! Deterministic SDK substitute preserves the submitted file set.
use crate::domain::diagnostics::{errors::DiagnosticsError as Error, types::RunReport};
use crate::domain::suite::types::WorkerRequest;
use crate::outbound::engine::interface::EnginePort;
#[derive(Clone, Copy, Debug, Default)]
/// Deterministic engine substitute for orchestration and contract tests.
pub struct EngineFake {
    /// Whether candidate evaluation returns the fixture refusal.
    pub refuses: bool,
}
impl EnginePort for EngineFake {
    fn run(&self, request: WorkerRequest) -> Result<RunReport, Error> {
        if self.refuses {
            return Err(Error::Invalid("no candidate queries".into()));
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
            diagnostics: Vec::new(),
        })
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
