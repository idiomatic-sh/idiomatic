//! Both contestants use the same execution implementation.
use super::interface::MeasureUseCase;
use crate::domain::measurements::types::Pair;
use crate::domain::suite::types::Audit;
use crate::domain::{diagnostics::errors::Error, suite::types::RunRequest};
use crate::outbound::{execution::interface::ExecutionPort, linter::interface::LinterPort};
use std::sync::Arc;
pub struct MeasureUseCaseImpl {
    reference: Arc<dyn LinterPort>,
    candidate: Arc<dyn LinterPort>,
    execution: Arc<dyn ExecutionPort>,
}
impl MeasureUseCaseImpl {
    pub fn new(
        reference: Arc<dyn LinterPort>,
        candidate: Arc<dyn LinterPort>,
        execution: Arc<dyn ExecutionPort>,
    ) -> Self {
        Self {
            reference,
            candidate,
            execution,
        }
    }
}
impl MeasureUseCase for MeasureUseCaseImpl {
    fn measure(&self, audit: &Audit, request: &RunRequest) -> Result<Pair, Error> {
        super::service::measure(
            audit,
            self.reference.as_ref(),
            self.candidate.as_ref(),
            self.execution.as_ref(),
            request,
        )
    }
}
