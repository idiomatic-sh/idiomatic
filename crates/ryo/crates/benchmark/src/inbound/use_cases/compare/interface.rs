//! Compare independently declared diagnostic and execution evidence.
use ryo_benchmark_contracts::domain::reports::types::Difference;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
/// Apply explicit contracts to finding, submitted-file and execution evidence.
pub trait CompareUseCase {
    /// Require equal findings, submitted inputs and explicitly complete execution metadata.
    ///
    /// # Errors
    ///
    /// Returns raw reports when a dimension differs or execution parity is unproven.
    fn compare(&self, reference: &RunReport, candidate: &RunReport) -> Result<(), Difference>;
}
