//! The worker orchestrates transport and SDK evaluation without reading wire data.
use super::interface::RunRyoUseCase;
use crate::outbound::{
    context::interface::ContextPort, engine::interface::EnginePort, worker::interface::WorkerPort,
};
use ryo_benchmark_contracts::domain::reports::types::ProjectContext;
use ryo_benchmark_contracts::domain::requests::types::ContextSeal;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::sync::Arc;
/// Handles one child-process request through independently injected boundaries.
pub struct RunRyoUseCaseImpl {
    /// SDK evaluation boundary selected by the language composition.
    engine: Arc<dyn EnginePort>,
    /// Child request and response transport boundary.
    worker: Arc<dyn WorkerPort>,
    /// Sealed project context validation and receipt boundary.
    context: Arc<dyn ContextPort>,
}
impl RunRyoUseCaseImpl {
    /// Bind the language-specific engine and shared process transport.
    #[must_use]
    pub fn new(
        engine: Arc<dyn EnginePort>,
        worker: Arc<dyn WorkerPort>,
        context: Arc<dyn ContextPort>,
    ) -> Self {
        Self {
            engine,
            worker,
            context,
        }
    }
}
impl RunRyoUseCase for RunRyoUseCaseImpl {
    fn run_ryo(&self) -> Result<(), Error> {
        let request = self.worker.receive()?;
        let prepared = crate::application::worker_request::service::prepare(request)?;
        if let ContextSeal::Sealed(manifest) = &prepared.request.context_manifest {
            self.context.validate(manifest, &prepared.request.root)?;
        }
        let session = self.engine.load(&prepared.request)?;
        let configured =
            crate::application::worker_request::service::configure(&prepared.request, session)?;
        let run = configured.execute()?;
        let mut report = run.report;
        if let ContextSeal::Sealed(manifest) = &prepared.request.context_manifest {
            report.project_context = ProjectContext::Observed(self.context.receipt(
                manifest,
                &prepared.request.root,
                run.dependencies,
            )?);
        }
        self.worker.respond(&report)
    }
}
impl std::fmt::Debug for RunRyoUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
