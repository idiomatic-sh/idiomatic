//! Readable suite orchestration, borrowing the actual injected port owners.
use super::interface::StageSuiteUseCase;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;
/// stage_suite with required borrowed boundaries.
pub struct StageSuiteUseCaseImpl<'a> {
    suite: &'a dyn crate::outbound::suite::interface::SuitePort,
}
impl<'a> StageSuiteUseCaseImpl<'a> {
    /// Borrow the boundaries already composed by the caller.
    #[must_use]
    pub fn new(suite: &'a dyn crate::outbound::suite::interface::SuitePort) -> Self {
        Self { suite }
    }
}
impl StageSuiteUseCase for StageSuiteUseCaseImpl<'_> {
    fn stage_suite(
        &self,
        directory: &Path,
        destination: &Path,
        authored: &[ryo::linter::RuleId],
        purpose: ryo_benchmark_contracts::domain::workloads::types::StagePurpose,
    ) -> Result<(), ReportsError> {
        crate::application::suite_staging::service::stage(
            self.suite,
            directory,
            destination,
            authored,
            purpose,
        )
    }
}
