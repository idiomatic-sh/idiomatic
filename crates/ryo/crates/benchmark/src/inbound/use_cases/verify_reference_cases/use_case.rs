//! Readable suite orchestration, borrowing the actual injected port owners.
use super::interface::VerifyReferenceCasesUseCase;
use benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;
/// verify_reference_cases with required borrowed boundaries.
pub struct VerifyReferenceCasesUseCaseImpl<'a> {
    suite: &'a dyn crate::outbound::suite::interface::SuitePort,
    host: &'a dyn crate::outbound::host::interface::HostPort,
    receipts: &'a dyn crate::outbound::receipts::interface::ReceiptsPort,
    context: &'a dyn crate::outbound::context::interface::ContextPort,
    reference: &'a dyn crate::outbound::linter::interface::LinterPort,
    execution: &'a dyn crate::outbound::execution::interface::ExecutionPort,
    inventory: &'a dyn crate::outbound::inventory::interface::InventoryPort,
    evidence: &'a dyn crate::outbound::evidence::interface::EvidencePort,
}
impl<'a> VerifyReferenceCasesUseCaseImpl<'a> {
    /// Borrow the boundaries already composed by the caller.
    #[must_use]
    pub fn new(
        suite: &'a dyn crate::outbound::suite::interface::SuitePort,
        host: &'a dyn crate::outbound::host::interface::HostPort,
        receipts: &'a dyn crate::outbound::receipts::interface::ReceiptsPort,
        context: &'a dyn crate::outbound::context::interface::ContextPort,
        reference: &'a dyn crate::outbound::linter::interface::LinterPort,
        execution: &'a dyn crate::outbound::execution::interface::ExecutionPort,
        inventory: &'a dyn crate::outbound::inventory::interface::InventoryPort,
        evidence: &'a dyn crate::outbound::evidence::interface::EvidencePort,
    ) -> Self {
        Self {
            suite,
            host,
            receipts,
            context,
            reference,
            execution,
            inventory,
            evidence,
        }
    }
}
impl VerifyReferenceCasesUseCase for VerifyReferenceCasesUseCaseImpl<'_> {
    fn verify_reference_cases(&self, rules: &Path, scratch: &Path) -> Result<benchmark_contracts::domain::workloads::types::reference_case_count::value::ReferenceCaseCount, ReportsError>{
        let cases = crate::application::case_preparation::service::cases(
            self.suite,
            self.context,
            rules,
            scratch,
        )?;
        let verified = super::service::verify_prepared_reference_cases(
            self.suite,
            self.host,
            self.receipts,
            self.context,
            self.reference,
            self.execution,
            self.inventory,
            rules,
            cases,
        )?;
        let tool = self
            .suite
            .profile(crate::outbound::suite::types::ArtifactPath(
                rules.parent().ok_or_else(|| {
                    ReportsError::Invalid(
                    crate::application::input_metadata::constants::MISSING_CASE_PROFILE_BOUNDARY
                        .into(),
                )
                })?,
            ))?;
        // The times file of a verification names the tool it verified against: the bench reads it back by rule, and its own
        // document names the language.
        if !verified.timings.is_empty() {
            self.evidence.verification(
                &tool.tool.0.as_str().into(),
                &crate::outbound::evidence::types::RuleTimings(verified.timings),
            )?;
            self.evidence.finish()?;
        }
        Ok(verified.count)
    }
}
