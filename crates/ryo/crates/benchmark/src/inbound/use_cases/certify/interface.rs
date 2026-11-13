//! Issue a sealed certificate for an exact independently verified workload.
use crate::domain::suite::types::certificate::value::Certificate;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
/// Establish case evidence and bind its complete execution inputs.
pub trait CertifyUseCase {
    /// Refuse inconsistent or changing evidence before any benchmark observation.
    ///
    /// # Errors
    /// Refuses incomplete inventory, fixture differences, changed artifacts or tool identity.
    #[expect(
        clippy::too_many_arguments,
        reason = "each path is an independently sealed identity; the shared use-case contract keeps these inputs explicit"
    )]
    fn certify(
        &self,
        directory: &std::path::Path,
        workload: &std::path::Path,
        reference_executable: &std::path::Path,
        candidate_executable: &std::path::Path,
        selected_rules: &std::path::Path,
        scratch: &std::path::Path,
        scope: ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage,
        cases: &ryo_benchmark_contracts::domain::requests::types::CertificationCases,
    ) -> Result<Certificate, Error>;
}
