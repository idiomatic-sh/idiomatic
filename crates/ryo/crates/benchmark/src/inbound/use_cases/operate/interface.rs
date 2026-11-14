//! What the `operate` use case promises its callers.
use super::types::{OperationReport, OperationRequest};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;

/// Run one gated operation of a linter bench.
pub trait OperateUseCase {
    /// Seal the bench's worker in a fresh output directory, stage what the operation needs and run it; the worker's
    /// standard output (a measure's or a parity's metrics) is copied to `output` as it arrives.
    ///
    /// # Errors
    /// Refuses a path that is not a bench directory, an absent reference tool, a failed build or registration,
    /// and a worker that cannot start.
    fn operate(
        &self,
        request: &OperationRequest,
        output: &mut dyn std::io::Write,
    ) -> Result<OperationReport, ReportsError>;
}
