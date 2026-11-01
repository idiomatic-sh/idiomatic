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
            project_context: None,
            files: request.files,
            skipped: Vec::new(),
            coverage: None,
            diagnostics: Vec::new(),
        })
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
