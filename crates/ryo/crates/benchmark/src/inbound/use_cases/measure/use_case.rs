//! Both contestants use the same execution implementation.
use super::interface::MeasureUseCase;
use crate::domain::measurements::types::{Pair, RunOrder};
use crate::domain::suite::types::audit::value::Audit;
use crate::domain::{diagnostics::errors::Error, suite::types::RunRequest};
use crate::outbound::{execution::interface::ExecutionPort, linter::interface::LinterPort};
use std::sync::Arc;
/// Release paired process observations only after the correctness gate succeeds.
pub struct MeasureUseCaseImpl {
    reference: Arc<dyn LinterPort>,
    candidate: Arc<dyn LinterPort>,
    execution: Arc<dyn ExecutionPort>,
}
impl MeasureUseCaseImpl {
    /// Construct this operation with the dependencies chosen by its composition.
    #[must_use]
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
    fn measure(&self, audit: &Audit, request: &RunRequest, order: RunOrder) -> Result<Pair, Error> {
        super::service::measure(
            audit,
            self.reference.as_ref(),
            self.candidate.as_ref(),
            self.execution.as_ref(),
            request,
            order,
        )
    }
}

impl std::fmt::Debug for MeasureUseCaseImpl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
