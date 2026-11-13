//! Public suite operation over exact typed artifact requests.
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;
/// verify_reference_cases through the injected suite boundaries.
pub trait VerifyReferenceCasesUseCase {
    /// # Errors
    /// Propagates exact artifact, admission and execution refusals.
    fn verify_reference_cases(&self, rules: &Path, scratch: &Path) -> Result<ryo_benchmark_contracts::domain::workloads::types::reference_case_count::value::ReferenceCaseCount, ReportsError>;
}
