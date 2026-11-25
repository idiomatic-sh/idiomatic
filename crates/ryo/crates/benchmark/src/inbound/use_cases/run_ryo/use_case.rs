//! The worker orchestrates transport and SDK evaluation without reading wire data.
use super::interface::RunRyoUseCase;
use crate::application::measurement::phases::{Clock, observation_file};
use crate::outbound::execution::types::{
    InProcessWorker, RuleCompileTimes, StoredRequest, WorkerResponse,
};
use crate::outbound::{
    context::interface::ContextPort, engine::interface::EnginePort, worker::interface::WorkerPort,
};
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::reports::types::{ProjectContext, RunReport};
use benchmark_contracts::domain::requests::types::{
    ContextSeal, DiagnosticSwitch, WorkerRequest, WorkerState,
};
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
    fn report(
        &self,
        request: WorkerRequest,
        mut clock: Option<&mut Clock>,
    ) -> Result<RunReport, Error> {
        let prepared = crate::application::worker_request::service::prepare(request)?;
        let session = self.engine.load(&prepared.request)?;
        if let Some(clock) = &mut clock {
            clock.boundary()?;
        }
        let configured =
            crate::application::worker_request::service::configure(&prepared.request, session)?;
        if let Some(clock) = &mut clock {
            clock.boundary()?;
        }
        let run = configured.execute()?;
        if let Some(clock) = &mut clock {
            clock.boundary()?;
        }
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
    /// The compile time of each rule of `request`, prepared, loaded and configured as a run is, through the configured session,
    /// so a rule that reads the relations a run declares or the rules it projects compiles as it does in the run.
    fn compiled(
        &self,
        request: WorkerRequest,
    ) -> Result<Option<std::collections::BTreeMap<ryo::linter::RuleId, std::time::Duration>>, Error>
    {
        let prepared = crate::application::worker_request::service::prepare(request)?;
        let session = self.engine.load(&prepared.request)?;
        let configured =
            crate::application::worker_request::service::configure(&prepared.request, session)?;
        configured.compile().transpose()
    }
}
impl RunRyoUseCase for RunRyoUseCaseImpl {
    fn run_ryo(&self) -> Result<(), Error> {
        let request = self.worker.receive()?;
        let phase_path = match &request.state {
            WorkerState::Retained(cache)
                if request.performance.diagnostics.execution == DiagnosticSwitch::On =>
            {
                cache.parent().and_then(observation_file)
            }
            _ => None,
        };
        let mut clock = phase_path.as_ref().map(|_| Clock::started()).transpose()?;
        let report = self.report(request, clock.as_mut())?;
        self.worker.respond(&report)?;
        if let Some((path, mut clock)) = phase_path.zip(clock) {
            clock.boundary()?;
            clock.publish(&path)?;
        }
        Ok(())
    }
}
impl InProcessWorker for RunRyoUseCaseImpl {
    fn run(&self, request: &StoredRequest) -> Result<WorkerResponse, Error> {
        let report = self.report(self.worker.read(&request.0)?, None)?;
        Ok(WorkerResponse(self.worker.encoded(&report)?.0))
    }
    fn compile(&self, request: &StoredRequest) -> Result<Option<RuleCompileTimes>, Error> {
        Ok(self
            .compiled(self.worker.read(&request.0)?)?
            .map(RuleCompileTimes))
    }
}
impl std::fmt::Debug for RunRyoUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
