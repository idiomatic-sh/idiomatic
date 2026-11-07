//! Readable suite orchestration, borrowing the actual injected port owners.
use super::interface::PrepareCasesUseCase;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;
/// prepare_cases with required borrowed boundaries.
pub struct PrepareCasesUseCaseImpl<'a> {
    suite: &'a dyn crate::outbound::suite::interface::SuitePort,
}
impl<'a> PrepareCasesUseCaseImpl<'a> {
    /// Borrow the boundaries already composed by the caller.
    #[must_use]
    pub fn new(suite: &'a dyn crate::outbound::suite::interface::SuitePort) -> Self {
        Self { suite }
    }
}
impl PrepareCasesUseCase for PrepareCasesUseCaseImpl<'_> {
    fn prepare_cases(
        &self,
        rules: &Path,
        scratch: &Path,
    ) -> Result<Vec<ryo_benchmark_contracts::domain::workloads::types::PreparedCase>, ReportsError>
    {
        crate::application::case_preparation::service::cases(self.suite, rules, scratch)
    }
}
