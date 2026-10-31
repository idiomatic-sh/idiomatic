//! The worker orchestrates transport and SDK evaluation without reading wire data.
use super::interface::RunRyoUseCase;
use crate::domain::diagnostics::errors::Error;
use crate::outbound::{engine::interface::EnginePort, worker::interface::WorkerPort};
use std::sync::Arc;
/// Handles one child-process request through independently injected boundaries.
pub struct RunRyoUseCaseImpl {
    engine: Arc<dyn EnginePort>,
    worker: Arc<dyn WorkerPort>,
}
impl RunRyoUseCaseImpl {
    /// Bind the language-specific engine and shared process transport.
    #[must_use]
    pub fn new(engine: Arc<dyn EnginePort>, worker: Arc<dyn WorkerPort>) -> Self {
        Self { engine, worker }
    }
}
impl RunRyoUseCase for RunRyoUseCaseImpl {
    fn run_ryo(&self) -> Result<(), Error> {
        let request = self.worker.receive()?;
        let report = self.engine.run(request)?;
        self.worker.respond(&report)
    }
}
impl std::fmt::Debug for RunRyoUseCaseImpl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
