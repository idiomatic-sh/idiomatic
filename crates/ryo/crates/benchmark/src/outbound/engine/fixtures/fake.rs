//! Deterministic SDK substitute preserves the submitted file set.
use crate::outbound::engine::interface::EnginePort;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
#[derive(Clone, Copy, Debug, Default)]
/// Deterministic engine substitute for orchestration and contract tests.
pub struct EngineFake {
    /// Whether candidate evaluation returns the fixture refusal.
    pub refuses: bool,
}
impl EnginePort for EngineFake {
    fn run(
        &self,
        request: ryo_benchmark_contracts::domain::requests::types::WorkerRequest,
        _prepared: crate::outbound::engine::types::PreparedMissingInputs,
    ) -> Result<RunReport, Error> {
        if self.refuses {
            return Err(Error::Invalid("no candidate queries".into()));
        }
        Ok(RunReport {
            execution_metadata:
                ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
            project_context: None,
            files: request.files.clone(),
            skipped: Vec::new(),
            coverage: Some(
                ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                    selected: request.files.len(),
                    evaluated: request.files.len(),
                    skipped: 0,
                    unknown_or_unread: 0,
                },
            ),
            diagnostics: Vec::new(),
        })
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
