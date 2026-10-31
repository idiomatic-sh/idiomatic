//! The SDK worker calls its injected engine boundary.
use super::interface::RunRyoUseCase;
use crate::domain::diagnostics::{errors::Error, types::RunReport};
use crate::domain::suite::types::WorkerRequest;
use crate::outbound::engine::interface::EnginePort;
use std::sync::Arc;
pub struct RunRyoUseCaseImpl {
    engine: Arc<dyn EnginePort>,
}
impl RunRyoUseCaseImpl {
    pub fn new(engine: Arc<dyn EnginePort>) -> Self {
        Self { engine }
    }
}
impl RunRyoUseCase for RunRyoUseCaseImpl {
    fn run_ryo(&self, request: WorkerRequest) -> Result<RunReport, Error> {
        self.engine.run(request)
    }
}
