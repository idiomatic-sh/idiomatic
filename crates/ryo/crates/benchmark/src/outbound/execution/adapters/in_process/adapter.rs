//! The worker's request runs in this process through the edge's worker; any other invocation runs on the process execution.
use super::constants::{COMPLETED, FAILED, RESPONSE_POISONED};
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::execution::types::{CompileTimes, ExitObservationMode, InProcessWorker};
use ryo_benchmark_contracts::domain::measurements::types::{
    CapturedBytes, ExecutionProfile, Invocation, ProcessExitCode, ProcessOutput,
    ResourceObservation, TerminationSignal,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::constants::RUN_RYO;
use std::sync::{Arc, Mutex};

/// Runs the worker's request through the edge's worker, in this process; any other invocation runs on `process`.
pub(crate) struct ExecutionInProcessAdapter {
    /// What every invocation but the worker's runs on.
    process: Box<dyn ExecutionPort>,
    /// The worker the edge composed over its engine.
    worker: InProcessWorker,
}
impl ExecutionInProcessAdapter {
    /// Bind the process execution and the edge's worker.
    pub(crate) fn new(process: Box<dyn ExecutionPort>, worker: InProcessWorker) -> Self {
        Self { process, worker }
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
            .map_err(|_poisoned| std::io::Error::other(RESPONSE_POISONED))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl ExecutionPort for ExecutionInProcessAdapter {
    fn exit_observation(&self) -> ExitObservationMode {
        self.process.exit_observation()
    }
    fn profile(&self) -> ExecutionProfile {
        self.process.profile()
    }
    fn compile(&self, invocation: &Invocation) -> CompileTimes {
        let path = Self::request_file(invocation)?;
        // A request the worker cannot read compiles nothing ahead: its run reports the same refusal as the run's failure.
        match (self.worker.compile)(&path) {
            Ok(times) => times,
            Err(_unread) => None,
        }
    }
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        let Some(path) = Self::request_file(&invocation) else {
            return self.process.run(invocation);
        };
        let response = Response::default();
        let started = std::time::Instant::now();
        let outcome = (self.worker.run)(&path, Box::new(response.clone()));
        let wall = started.elapsed();
        let stdout = response
            .0
            .lock()
            .map_err(|_poisoned| Error::Invalid(RESPONSE_POISONED.into()))?
            .clone();
        let (status, stderr) = match outcome {
            Ok(()) => (COMPLETED, Vec::new()),
            Err(error) => (FAILED, error.to_string().into_bytes()),
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
