//! Readable suite orchestration, borrowing the actual injected port owners.
use super::interface::VerifyCasesUseCase;
use crate::application::input_metadata::constants;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;
/// verify_cases with required borrowed boundaries.
pub struct VerifyCasesUseCaseImpl<'a> {
    suite: &'a dyn crate::outbound::suite::interface::SuitePort,
    receipts: &'a dyn crate::outbound::receipts::interface::ReceiptsPort,
    context: &'a dyn crate::outbound::context::interface::ContextPort,
    reference: &'a dyn crate::outbound::linter::interface::LinterPort,
    candidate: &'a dyn crate::outbound::linter::interface::LinterPort,
    execution: &'a dyn crate::outbound::execution::interface::ExecutionPort,
    inventory: &'a dyn crate::outbound::inventory::interface::InventoryPort,
}
impl<'a> VerifyCasesUseCaseImpl<'a> {
    /// Borrow the boundaries already composed by the caller.
    #[must_use]
    pub fn new(
        suite: &'a dyn crate::outbound::suite::interface::SuitePort,
        receipts: &'a dyn crate::outbound::receipts::interface::ReceiptsPort,
        context: &'a dyn crate::outbound::context::interface::ContextPort,
        reference: &'a dyn crate::outbound::linter::interface::LinterPort,
        candidate: &'a dyn crate::outbound::linter::interface::LinterPort,
        execution: &'a dyn crate::outbound::execution::interface::ExecutionPort,
        inventory: &'a dyn crate::outbound::inventory::interface::InventoryPort,
    ) -> Self {
        Self {
            suite,
            receipts,
            context,
            reference,
            candidate,
            execution,
            inventory,
        }
    }
}
impl VerifyCasesUseCase for VerifyCasesUseCaseImpl<'_> {
    fn verify_cases(
        &self,
        rules: &Path,
        scratch: &Path,
        selection: ryo_benchmark_contracts::domain::requests::types::CaseSelection<'_>,
        coverage: ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage,
    ) -> Result<
        ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount,
        ReportsError,
    > {
        if coverage
            == ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage::Accounted
        {
            let profile = rules.parent().ok_or_else(|| {
                ReportsError::Invalid(constants::MISSING_CASE_PROFILE_BOUNDARY.into())
            })?;
            let tool = self
                .suite
                .profile(crate::outbound::suite::types::ArtifactPath(profile))?;
            let audit =
                crate::application::suite_auditing::service::audit(self.suite, profile, &tool)?;
            crate::domain::suite::service::require_accounted(&audit, &tool)?;
        }
        let cases = crate::application::case_preparation::service::cases(
            self.suite,
            self.context,
            rules,
            scratch,
        )?;
        super::service::verify_prepared_shard(
            self.suite,
            self.receipts,
            self.context,
            self.reference,
            self.candidate,
            self.execution,
            self.inventory,
            rules,
            cases,
            selection,
        )
    }
}
