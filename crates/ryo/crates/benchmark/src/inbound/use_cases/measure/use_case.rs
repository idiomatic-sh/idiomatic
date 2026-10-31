//! Both contestants use the same execution implementation.
use super::interface::MeasureUseCase;
use crate::domain::measurements::types::{RunOrder, pair::value::Pair};
use crate::domain::suite::types::certificate::value::Certificate;
use crate::domain::{diagnostics::errors::DiagnosticsError as Error, suite::types::RunRequest};
use crate::outbound::suite::interface::SuitePort;
use crate::outbound::{execution::interface::ExecutionPort, linter::interface::LinterPort};
use std::sync::Arc;
/// Release paired process observations only after the correctness gate succeeds.
pub struct MeasureUseCaseImpl {
    /// Pinned reference tool preparation and decoding boundary.
    reference: Arc<dyn LinterPort>,
    /// Candidate tool preparation and decoding boundary.
    candidate: Arc<dyn LinterPort>,
    /// Process launch and resource observation boundary.
    execution: Arc<dyn ExecutionPort>,
    /// Artifact validation and exact content fingerprinting boundary.
    suite: Arc<dyn SuitePort>,
}
impl MeasureUseCaseImpl {
    /// Construct this operation with the dependencies chosen by its composition.
    #[must_use]
    pub fn new(
        reference: Arc<dyn LinterPort>,
        candidate: Arc<dyn LinterPort>,
        execution: Arc<dyn ExecutionPort>,
        suite: Arc<dyn SuitePort>,
    ) -> Self {
        Self {
            reference,
            candidate,
            execution,
            suite,
        }
    }
}
impl MeasureUseCase for MeasureUseCaseImpl {
    fn measure(
        &self,
        certificate: &Certificate,
        request: &RunRequest,
        order: RunOrder,
    ) -> Result<Pair, Error> {
        super::service::measure_certified(
            certificate,
            self.suite.as_ref(),
            self.reference.as_ref(),
            self.candidate.as_ref(),
            self.execution.as_ref(),
            request,
            order,
        )
    }
}

impl std::fmt::Debug for MeasureUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
