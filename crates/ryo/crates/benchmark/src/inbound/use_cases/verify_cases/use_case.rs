//! Readable suite orchestration, borrowing the actual injected port owners.
use super::interface::VerifyCasesUseCase;
use crate::application::input_metadata::constants;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;
/// verify_cases with required borrowed boundaries.
pub struct VerifyCasesUseCaseImpl<'a> {
    suite: &'a dyn crate::outbound::suite::interface::SuitePort,
    host: &'a dyn crate::outbound::host::interface::HostPort,
    receipts: &'a dyn crate::outbound::receipts::interface::ReceiptsPort,
    context: &'a dyn crate::outbound::context::interface::ContextPort,
    reference: &'a dyn crate::outbound::linter::interface::LinterPort,
    candidate: &'a dyn crate::outbound::linter::interface::LinterPort,
    executions: super::types::Executions<'a>,
    inventory: &'a dyn crate::outbound::inventory::interface::InventoryPort,
    evidence: &'a dyn crate::outbound::evidence::interface::EvidencePort,
}
impl<'a> VerifyCasesUseCaseImpl<'a> {
    /// Borrow the boundaries already composed by the caller.
    #[must_use]
    pub fn new(
        suite: &'a dyn crate::outbound::suite::interface::SuitePort,
        host: &'a dyn crate::outbound::host::interface::HostPort,
        receipts: &'a dyn crate::outbound::receipts::interface::ReceiptsPort,
        context: &'a dyn crate::outbound::context::interface::ContextPort,
        reference: &'a dyn crate::outbound::linter::interface::LinterPort,
        candidate: &'a dyn crate::outbound::linter::interface::LinterPort,
        executions: super::types::Executions<'a>,
        inventory: &'a dyn crate::outbound::inventory::interface::InventoryPort,
        evidence: &'a dyn crate::outbound::evidence::interface::EvidencePort,
    ) -> Self {
        Self {
            suite,
            host,
            receipts,
            context,
            reference,
            candidate,
            executions,
            inventory,
            evidence,
        }
    }
}
impl VerifyCasesUseCase for VerifyCasesUseCaseImpl<'_> {
    fn verify_cases(
        &self,
        rules: &Path,
        scratch: &Path,
        selection: ryo_benchmark_contracts::domain::requests::types::CaseSelection,
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
        let verified = super::service::verify_prepared_shard(
            self.suite,
            self.host,
            self.receipts,
            self.context,
            self.reference,
            self.candidate,
            self.executions,
            self.inventory,
            rules,
            cases,
            selection,
        )?;
        let tool = self
            .suite
            .profile(crate::outbound::suite::types::ArtifactPath(
                rules.parent().ok_or_else(|| {
                    ReportsError::Invalid(constants::MISSING_CASE_PROFILE_BOUNDARY.into())
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
