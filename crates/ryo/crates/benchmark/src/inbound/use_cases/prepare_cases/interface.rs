//! Public suite operation over exact typed artifact requests.
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;
/// prepare_cases through the injected suite boundaries.
pub trait PrepareCasesUseCase {
    /// # Errors
    /// Propagates exact artifact, admission and execution refusals.
    fn prepare_cases(
        &self,
        rules: &Path,
        scratch: &Path,
    ) -> Result<Vec<ryo_benchmark_contracts::domain::workloads::types::PreparedCase>, ReportsError>;
}
