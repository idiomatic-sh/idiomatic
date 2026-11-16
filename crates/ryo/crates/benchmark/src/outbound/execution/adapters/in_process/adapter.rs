//! A worker request runs in this process; any other invocation runs as the process execution runs it.
use crate::outbound::context::interface::ContextPort;
use crate::outbound::engine::interface::EnginePort;
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::worker::{factory::json, interface::WorkerPort as _, types::WorkerSource};
use ryo_benchmark_contracts::domain::measurements::types::{
    CapturedBytes, ExecutionProfile, Invocation, ProcessExitCode, ProcessOutput,
    ResourceObservation, TerminationSignal,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::constants::RUN_RYO;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Runs the worker's request over the engine it was given, in this process.
pub(crate) struct ExecutionInProcessAdapter {
    /// What every invocation but the worker's runs on.
    process: Box<dyn ExecutionPort>,
    /// The engine every worker request runs over.
    engine: Arc<dyn EnginePort + Send + Sync>,
    /// Sealed project context validation and receipt boundary.
    context: Arc<dyn ContextPort + Send + Sync>,
}
impl ExecutionInProcessAdapter {
    /// Bind the process execution, the engine and the context.
    pub(crate) fn new(
        process: Box<dyn ExecutionPort>,
        engine: Arc<dyn EnginePort + Send + Sync>,
        context: Arc<dyn ContextPort + Send + Sync>,
    ) -> Self {
        Self {
            process,
            engine,
            context,
        }
    }
    /// The stored request a worker invocation names, none for any other invocation.
    fn request_file(invocation: &Invocation) -> Option<std::path::PathBuf> {
        let mut arguments = invocation.arguments.iter();
        (arguments.next()?.to_str()? == RUN_RYO).then_some(())?;
        let _flag = arguments.next()?;
        Some(std::path::PathBuf::from(arguments.next()?))
    }
}
impl std::fmt::Debug for ExecutionInProcessAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
/// The bytes the worker's response is written to, which the invocation's output then holds.
#[derive(Clone, Default)]
struct Response(Arc<Mutex<Vec<u8>>>);
impl std::io::Write for Response {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_poisoned| std::io::Error::other("response lock poisoned"))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl ExecutionPort for ExecutionInProcessAdapter {
    fn exit_observation(&self) -> crate::outbound::execution::types::ExitObservationMode {
        self.process.exit_observation()
    }
    fn profile(&self) -> ExecutionProfile {
        self.process.profile()
    }
    fn compile(
        &self,
        invocation: &Invocation,
    ) -> Result<BTreeMap<ryo::linter::RuleId, std::time::Duration>, Error> {
        let Some(path) = Self::request_file(invocation) else {
            return Ok(BTreeMap::new());
        };
        let request = json(WorkerSource::Stored(path), Box::new(Response::default())).receive()?;
        self.engine.compile(&request)
    }
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        let Some(path) = Self::request_file(&invocation) else {
            return self.process.run(invocation);
        };
        let response = Response::default();
        let transport = json(WorkerSource::Stored(path), Box::new(response.clone()));
        let started = std::time::Instant::now();
        let outcome = crate::inbound::use_cases::run_ryo::use_case::RunRyoUseCaseImpl::new(
            Arc::clone(&self.engine) as Arc<dyn EnginePort>,
            Arc::new(transport),
            Arc::clone(&self.context) as Arc<dyn ContextPort>,
        );
        let outcome =
            crate::inbound::use_cases::run_ryo::interface::RunRyoUseCase::run_ryo(&outcome);
        let wall = started.elapsed();
        let stdout = response
            .0
            .lock()
            .map_err(|_poisoned| Error::Invalid("response lock poisoned".into()))?
            .clone();
        let (status, stderr) = match outcome {
            Ok(()) => (0, Vec::new()),
            Err(error) => (1, error.to_string().into_bytes()),
        };
        Ok(ProcessOutput {
            signal: TerminationSignal::Exited,
            status: ProcessExitCode(status),
            stdout: CapturedBytes(stdout),
            stderr: CapturedBytes(stderr),
            wall,
            resources: ResourceObservation::WallOnly,
        })
    }
}
