//! Execute the candidate through the public SDK.
use crate::domain::diagnostics::{errors::DiagnosticsError as Error, types::RunReport};
use crate::domain::suite::types::WorkerRequest;
/// Execute candidate rules through the public SDK.
pub trait EnginePort {
    /// Evaluate a worker request and return its complete canonical report.
    ///
    /// # Errors
    ///
    /// Returns an error when inputs, rule evaluation or the resulting report are invalid.
    fn run(&self, request: WorkerRequest) -> Result<RunReport, Error>;
}
