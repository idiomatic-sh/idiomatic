//! The worker orchestrates transport and SDK evaluation without reading wire data.
use super::interface::RunRyoUseCase;
use crate::outbound::execution::types::{
    InProcessWorker, RuleCompileTimes, StoredRequest, WorkerResponse,
};
use crate::outbound::{
    context::interface::ContextPort, engine::interface::EnginePort, worker::interface::WorkerPort,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{ProjectContext, RunReport};
use ryo_benchmark_contracts::domain::requests::types::{ContextSeal, WorkerRequest};
use std::sync::Arc;
/// Handles one child-process request through independently injected boundaries; run in the bench's own process, it is the
/// in-process execution's worker, each stored request read and answered through the same transport.
pub struct RunRyoUseCaseImpl {
    /// SDK evaluation boundary selected by the language composition.
    engine: Arc<dyn EnginePort + Send + Sync>,
    /// Child request and response transport boundary.
    worker: Arc<dyn WorkerPort + Send + Sync>,
    /// Sealed project context validation and receipt boundary.
    context: Arc<dyn ContextPort + Send + Sync>,
}
impl RunRyoUseCaseImpl {
    /// Bind the language-specific engine and shared process transport.
    #[must_use]
    pub fn new(
        engine: Arc<dyn EnginePort + Send + Sync>,
        worker: Arc<dyn WorkerPort + Send + Sync>,
        context: Arc<dyn ContextPort + Send + Sync>,
    ) -> Self {
        Self {
            engine,
            worker,
            context,
        }
    }
}
impl RunRyoUseCaseImpl {
    /// The report of `request`: prepared, loaded, configured and run, with its sealed context's receipt.
    fn report(&self, request: WorkerRequest) -> Result<RunReport, Error> {
        let prepared = crate::application::worker_request::service::prepare(request)?;
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
        Ok(report)
    }
}
impl RunRyoUseCase for RunRyoUseCaseImpl {
    fn run_ryo(&self) -> Result<(), Error> {
        let request = self.worker.receive()?;
        let report = self.report(request)?;
        self.worker.respond(&report)
    }
}
impl InProcessWorker for RunRyoUseCaseImpl {
    fn run(&self, request: &StoredRequest) -> Result<WorkerResponse, Error> {
        let report = self.report(self.worker.read(&request.0)?)?;
        Ok(WorkerResponse(self.worker.encoded(&report)?.0))
    }
    fn compile(&self, request: &StoredRequest) -> Result<RuleCompileTimes, Error> {
        let read = self.worker.read(&request.0)?;
        // An engine that compiles inside each run compiles nothing ahead; one that cannot compile says why.
        Ok(RuleCompileTimes(
            self.engine.compile(&read).transpose()?.unwrap_or_default(),
        ))
    }
}
impl std::fmt::Debug for RunRyoUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

