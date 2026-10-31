//! The SDK worker calls its injected engine boundary.
use super::interface::RunRyoUseCase;
use crate::domain::diagnostics::{errors::Error, types::RunReport};
use crate::domain::suite::types::WorkerRequest;
use crate::outbound::engine::interface::EnginePort;
use std::sync::Arc;
/// Run the candidate worker through its injected SDK boundary.
pub struct RunRyoUseCaseImpl {
    engine: Arc<dyn EnginePort>,
}
impl RunRyoUseCaseImpl {
    /// Construct this operation with the dependencies chosen by its composition.
    #[must_use]
    pub fn new(engine: Arc<dyn EnginePort>) -> Self {
        Self { engine }
    }
}
impl RunRyoUseCase for RunRyoUseCaseImpl {
    fn run_ryo(&self, request: WorkerRequest) -> Result<RunReport, Error> {
        self.engine.run(request)
    }
}

impl std::fmt::Debug for RunRyoUseCaseImpl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
