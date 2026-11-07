//! The identity check receives through the worker transport and delegates the comparison.
use super::{interface::CheckWorkerRequestUseCase, types::WorkerExpectation};
use crate::outbound::worker::interface::WorkerPort;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::sync::Arc;
/// Checks one child-process request through the injected transport.
pub struct CheckWorkerRequestUseCaseImpl {
    /// Child request and response transport boundary.
    worker: Arc<dyn WorkerPort>,
}
impl CheckWorkerRequestUseCaseImpl {
    /// Bind the transport the worker's request arrives through.
    #[must_use]
    pub fn new(worker: Arc<dyn WorkerPort>) -> Self {
        Self { worker }
    }
}
impl CheckWorkerRequestUseCase for CheckWorkerRequestUseCaseImpl {
    fn check_worker_request(&self, expectation: &WorkerExpectation) -> Result<(), Error> {
        let request = self.worker.receive()?;
        crate::application::worker_identity::service::check(
            &request,
            &expectation.parser_configuration,
            &expectation.pinned_applicability,
        )
    }
}
impl std::fmt::Debug for CheckWorkerRequestUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
