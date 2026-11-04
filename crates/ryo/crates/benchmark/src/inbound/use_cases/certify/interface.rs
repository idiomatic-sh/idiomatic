//! Issue a sealed certificate for an exact independently verified workload.
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error, suite::types::certificate::value::Certificate,
};
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
        scope: crate::domain::suite::types::BenchmarkCoverage,
        cases: &crate::domain::suite::types::CertificationCases,
    ) -> Result<Certificate, Error>;
}
