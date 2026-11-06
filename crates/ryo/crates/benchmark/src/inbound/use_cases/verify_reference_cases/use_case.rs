//! Readable suite orchestration, borrowing the actual injected port owners.
use super::interface::VerifyReferenceCasesUseCase;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;
/// verify_reference_cases with required borrowed boundaries.
pub struct VerifyReferenceCasesUseCaseImpl<'a> {
    suite: &'a dyn crate::outbound::suite::interface::SuitePort,
    reference: &'a dyn crate::outbound::linter::interface::LinterPort,
    execution: &'a dyn crate::outbound::execution::interface::ExecutionPort,
    inventory: &'a dyn crate::outbound::inventory::interface::InventoryPort,
}
impl<'a> VerifyReferenceCasesUseCaseImpl<'a> {
    /// Borrow the boundaries already composed by the caller.
    #[must_use]
    pub fn new(
        suite: &'a dyn crate::outbound::suite::interface::SuitePort,
        reference: &'a dyn crate::outbound::linter::interface::LinterPort,
        execution: &'a dyn crate::outbound::execution::interface::ExecutionPort,
        inventory: &'a dyn crate::outbound::inventory::interface::InventoryPort,
    ) -> Self {
        Self {
            suite,
            reference,
            execution,
            inventory,
        }
    }
}
impl VerifyReferenceCasesUseCase for VerifyReferenceCasesUseCaseImpl<'_> {
    fn verify_reference_cases(&self, rules: &Path, scratch: &Path) -> Result<ryo_benchmark_contracts::domain::workloads::types::reference_case_count::value::ReferenceCaseCount, ReportsError>{
        let cases =
            crate::application::case_preparation::service::cases(self.suite, rules, scratch)?;
        crate::application::run::service::verify_prepared_reference_cases(
            self.suite,
            self.reference,
            self.execution,
            self.inventory,
            rules,
            cases,
        )
    }
}
