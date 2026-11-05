//! Execute the candidate through the public SDK.
use super::types::PreparedMissingInputs;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
/// Execute candidate rules through the public SDK.
pub trait EnginePort {
    /// Evaluate a worker request and return its complete canonical report.
    ///
    /// # Errors
    ///
    /// Returns an error when inputs, rule evaluation or the resulting report are invalid.
    fn run(
        &self,
        request: WorkerRequest,
        prepared: PreparedMissingInputs,
    ) -> Result<RunReport, Error>;
}
