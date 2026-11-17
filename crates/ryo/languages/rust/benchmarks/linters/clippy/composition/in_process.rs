//! A verification's candidate runs inside this process over one engine, and its rule times go to the file the run names.
use ryo_benchmark::inbound::use_cases::run_ryo::{
    interface::RunRyoUseCase as _, use_case::RunRyoUseCaseImpl,
};
use ryo_benchmark::inbound::use_cases::select_cases::types::{CaseRun, VerifyTimes};
use ryo_benchmark::outbound::context::interface::ContextPort;
use ryo_benchmark::outbound::engine::interface::EnginePort;
use ryo_benchmark::outbound::evidence::interface::EvidencePort;
use ryo_benchmark::outbound::execution::interface::ExecutionPort;
use ryo_benchmark::outbound::worker::{
    factory::json, interface::WorkerPort as _, types::WorkerSource,
};
use ryo_benchmark_contracts::domain::measurements::types::{
    CapturedBytes, ExecutionProfile, Invocation, ProcessExitCode, ProcessOutput,
    ResourceObservation, TerminationSignal,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::constants::RUN_RYO;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Runs the worker's request over the engine it was given, in this process; any other invocation runs on `process`.
pub(crate) struct InProcessExecution {
    /// What every invocation but the worker's runs on.
    process: Box<dyn ExecutionPort>,
    /// The engine every worker request runs over.
    engine: Arc<dyn EnginePort + Send + Sync>,
    /// Sealed project context validation and receipt boundary.
    context: Arc<dyn ContextPort + Send + Sync>,
}
impl InProcessExecution {
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
impl std::fmt::Debug for InProcessExecution {
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
            .map_err(|_poisoned| std::io::Error::other(super::constants::RESPONSE_POISONED))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl ExecutionPort for InProcessExecution {
    fn exit_observation(&self) -> ryo_benchmark::outbound::execution::types::ExitObservationMode {
        self.process.exit_observation()
    }
    fn profile(&self) -> ExecutionProfile {
        self.process.profile()
    }
    fn compile(
        &self,
        invocation: &Invocation,
    ) -> Option<BTreeMap<ryo::linter::RuleId, std::time::Duration>> {
        let path = Self::request_file(invocation)?;
        let engine: Arc<dyn EnginePort> = self.engine.clone();
        let context: Arc<dyn ContextPort> = self.context.clone();
        let transport = json(WorkerSource::Stored(path), Box::new(Response::default()));
        RunRyoUseCaseImpl::new(engine, Arc::new(transport), context).compile()
    }
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        let Some(path) = Self::request_file(&invocation) else {
            return self.process.run(invocation);
        };
        let response = Response::default();
        let transport = json(WorkerSource::Stored(path), Box::new(response.clone()));
        let engine: Arc<dyn EnginePort> = self.engine.clone();
        let context: Arc<dyn ContextPort> = self.context.clone();
        let started = std::time::Instant::now();
        let outcome = RunRyoUseCaseImpl::new(engine, Arc::new(transport), context).run_ryo();
        let wall = started.elapsed();
        let stdout = response
            .0
            .lock()
            .map_err(|_poisoned| Error::Invalid(super::constants::RESPONSE_POISONED.into()))?
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

/// Where the run's rule times go: the file it names, appended to, or nowhere.
///
/// # Errors
/// Refuses a file that cannot be opened for appending.
pub(crate) fn rule_times(run: &CaseRun) -> Result<impl EvidencePort, Error> {
    let output: Box<dyn std::io::Write + Send> = match &run.timings {
        VerifyTimes::Recorded(path) => Box::new(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?,
        ),
        VerifyTimes::Unrecorded => Box::new(std::io::sink()),
    };
    Ok(ryo_benchmark::outbound::evidence::factory::bmf(
        output,
        Box::new(std::io::sink()),
    ))
}
