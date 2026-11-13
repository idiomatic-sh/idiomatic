//! Public suite operation over exact typed artifact requests.
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;
/// stage_suite through the injected suite boundaries.
pub trait StageSuiteUseCase {
    /// # Errors
    /// Propagates exact artifact, admission and execution refusals.
    fn stage_suite(
        &self,
        directory: &Path,
        destination: &Path,
        authored: &[ryo::linter::RuleId],
        purpose: ryo_benchmark_contracts::domain::workloads::types::StagePurpose,
    ) -> Result<(), ReportsError>;
}
