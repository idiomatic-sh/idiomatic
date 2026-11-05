//! Public suite operation over exact typed artifact requests.
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;
/// verify_cases through the injected suite boundaries.
pub trait VerifyCasesUseCase {
    /// # Errors
    /// Propagates exact artifact, admission and execution refusals.
    fn verify_cases(
        &self,
        rules: &Path,
        scratch: &Path,
        selection: ryo_benchmark_contracts::domain::requests::types::CaseSelection<'_>,
        coverage: ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage,
    ) -> Result<
        ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount,
        ReportsError,
    >;
}
