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

    /// Apply the caller's explicit diagnostic or complete-report contract and retain both comparison dimensions.
    ///
    /// # Errors
    /// Refuses invalid candidate execution and differences required by the selected contract.
    fn compare_with_contract(
        &self,
        reference: &RunReport,
        candidate: &RunReport,
        contract: ryo_benchmark_contracts::domain::reports::types::ComparisonContract,
    ) -> Result<ryo_benchmark_contracts::domain::reports::types::ReportComparison, Difference>;
}
