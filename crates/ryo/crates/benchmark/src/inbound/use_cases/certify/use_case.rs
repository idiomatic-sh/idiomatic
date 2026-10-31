//! Certification owns the workflow over suite, linter and execution contracts.
use super::interface::CertifyUseCase;
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error, suite::types::certificate::value::Certificate,
};
use crate::outbound::{
    execution::interface::ExecutionPort, linter::interface::LinterPort, suite::interface::SuitePort,
};
use std::sync::Arc;
/// Establish independent correctness evidence for the selected execution profile.
pub struct CertifyUseCaseImpl {
    suite: Arc<dyn SuitePort>,
    reference: Arc<dyn LinterPort>,
    candidate: Arc<dyn LinterPort>,
    execution: Arc<dyn ExecutionPort>,
}
impl CertifyUseCaseImpl {
    /// Bind the four independently replaceable boundaries.
    pub fn new(
        suite: Arc<dyn SuitePort>,
        reference: Arc<dyn LinterPort>,
        candidate: Arc<dyn LinterPort>,
        execution: Arc<dyn ExecutionPort>,
    ) -> Self {
        Self {
            suite,
            reference,
            candidate,
            execution,
        }
    }
}
impl CertifyUseCase for CertifyUseCaseImpl {
    fn certify(
        &self,
        directory: &std::path::Path,
        workload: &std::path::Path,
        reference_executable: &std::path::Path,
        candidate_executable: &std::path::Path,
        selected_rules: &std::path::Path,
        scratch: &std::path::Path,
        scope: crate::domain::suite::types::BenchmarkCoverage,
    ) -> Result<Certificate, Error> {
        super::service::certify(
            self.suite.as_ref(),
            self.reference.as_ref(),
            self.candidate.as_ref(),
            self.execution.as_ref(),
            directory,
            workload,
            reference_executable,
            candidate_executable,
            selected_rules,
            scratch,
            scope,
        )
    }
}
impl std::fmt::Debug for CertifyUseCaseImpl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
