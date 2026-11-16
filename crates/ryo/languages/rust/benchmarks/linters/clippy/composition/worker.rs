//! The linter's in-process worker: each stored request the bench's verification sends the candidate runs through the
//! bench's one engine, in this process, as a worker child would run it.
use ryo_benchmark::inbound::use_cases::run_ryo::{
    interface::RunRyoUseCase as _, use_case::RunRyoUseCaseImpl,
};
use ryo_benchmark::outbound::context::interface::ContextPort;
use ryo_benchmark::outbound::engine::interface::EnginePort;
use ryo_benchmark::outbound::execution::types::{
    InProcessWorker, RuleCompileTimes, StoredRequest, WorkerResponse,
};
use ryo_benchmark::outbound::worker::{interface::WorkerPort as _, types::WorkerSource};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::sync::{Arc, Mutex};

/// The candidate's worker in this process: each stored request runs through the bench's one engine, which compiles each
/// rule once and keeps its plans for every case.
pub(super) struct InProcessRyo {
    /// The engine every request runs on.
    pub(super) engine: Arc<dyn EnginePort + Send + Sync>,
    /// Sealed project context validation and receipts.
    pub(super) context: Arc<dyn ContextPort + Send + Sync>,
}
impl std::fmt::Debug for InProcessRyo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// The bytes a request's response is written to.
#[derive(Clone, Default)]
struct Response(Arc<Mutex<Vec<u8>>>);
impl std::io::Write for Response {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_poisoned| std::io::Error::other(super::constants::RESPONSE_POISONED))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl InProcessWorker for InProcessRyo {
    fn run(&self, request: &StoredRequest) -> Result<WorkerResponse, Error> {
        let response = Response::default();
        RunRyoUseCaseImpl::new(
            Arc::<dyn EnginePort + Send + Sync>::clone(&self.engine),
            Arc::new(ryo_benchmark::outbound::worker::factory::json(
                WorkerSource::Stored(request.0.clone()),
                Box::new(response.clone()),
            )),
            Arc::<dyn ContextPort + Send + Sync>::clone(&self.context),
        )
        .run_ryo()?;
        let written = response
            .0
            .lock()
            .map_err(|_poisoned| Error::Invalid(super::constants::RESPONSE_POISONED.into()))?
            .clone();
        Ok(WorkerResponse(written))
    }
    fn compile(&self, request: &StoredRequest) -> Result<RuleCompileTimes, Error> {
        let read = ryo_benchmark::outbound::worker::factory::json(
            WorkerSource::Stored(request.0.clone()),
            Box::new(std::io::sink()),
        )
        .receive()?;
        Ok(RuleCompileTimes(
            EnginePort::compile(self.engine.as_ref(), &read).unwrap_or_default(),
        ))
    }
}
