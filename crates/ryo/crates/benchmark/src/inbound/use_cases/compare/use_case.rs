//! Exact normalized comparison has no external dependencies.
use super::interface::CompareUseCase;
use ryo_benchmark_contracts::domain::reports::types::Difference;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
/// Compare exact finding multisets and submitted-file coverage.
#[derive(Clone, Copy, Debug, Default)]
pub struct CompareUseCaseImpl;
impl CompareUseCaseImpl {
    /// Construct this operation with the dependencies chosen by its composition.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}
impl CompareUseCase for CompareUseCaseImpl {
    fn compare_with_contract(
        &self,
        reference: &RunReport,
        candidate: &RunReport,
        contract: ryo_benchmark_contracts::domain::reports::types::ComparisonContract,
    ) -> Result<ryo_benchmark_contracts::domain::reports::types::ReportComparison, Difference> {
        crate::domain::diagnostics::service::compare_with_contract(reference, candidate, contract)
    }

    fn compare(&self, reference: &RunReport, candidate: &RunReport) -> Result<(), Difference> {
        super::service::compare(reference, candidate)
    }
}
