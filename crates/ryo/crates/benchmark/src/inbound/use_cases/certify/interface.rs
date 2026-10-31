//! Issue a sealed certificate for an exact independently verified workload.
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error, suite::types::certificate::value::Certificate,
};
/// Establish case evidence and bind its complete execution inputs.
pub trait CertifyUseCase {
    /// Refuse inconsistent or changing evidence before any benchmark observation.
    fn certify(
        &self,
        directory: &std::path::Path,
        workload: &std::path::Path,
        reference_executable: &std::path::Path,
        candidate_executable: &std::path::Path,
        selected_rules: &std::path::Path,
        scratch: &std::path::Path,
        scope: crate::domain::suite::types::BenchmarkCoverage,
    ) -> Result<Certificate, Error>;
}
