//! A worker's request runs in this process through the language's worker; the composition chooses this execution for the
//! candidate, and any invocation that is no worker request is refused.
use super::constants::{COMPLETED, FAILED, NOT_A_WORKER_REQUEST, RESPONSE_POISONED};
use crate::outbound::execution::interface::{ExecutionPort, InProcessWorker};
use crate::outbound::execution::types::StoredRequest;
use ryo_benchmark_contracts::domain::measurements::types::{
    CapturedBytes, ExecutionProfile, Invocation, ProcessExitCode, ProcessOutput,
    ResourceObservation, TerminationSignal,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::constants::RUN_RYO;
use std::sync::{Arc, Mutex};

/// Runs each worker request through the language's worker, in this process.
#[derive(Debug)]
pub(crate) struct ExecutionInProcessAdapter<'a> {
    /// The worker the bench composed over its engine.
    worker: &'a dyn InProcessWorker,
}
impl<'a> ExecutionInProcessAdapter<'a> {
    /// Bind the language's worker.
    pub(crate) fn new(worker: &'a dyn InProcessWorker) -> Self {
        Self { worker }
    }
    /// The stored request a worker invocation names, none for any other invocation.
    fn request_file(invocation: &Invocation) -> Option<StoredRequest> {
        let mut arguments = invocation.arguments.iter();
        (arguments.next()?.to_str()? == RUN_RYO).then_some(())?;
        let _flag = arguments.next()?;
        Some(StoredRequest(std::path::PathBuf::from(arguments.next()?)))
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
impl ExecutionPort for ExecutionInProcessAdapter<'_> {
    fn profile(&self) -> ExecutionProfile {
        ExecutionProfile::Wall
    }
    fn compile(
        &self,
        invocation: &Invocation,
    ) -> Option<std::collections::BTreeMap<ryo::linter::RuleId, std::time::Duration>> {
        let request = Self::request_file(invocation)?;
        // A request the worker cannot read compiles nothing ahead: its run reports the same refusal as the run's failure.
        match self.worker.compile(&request) {
            Ok(times) => Some(times.0),
            Err(_unread) => None,
        }
    }
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        let request = Self::request_file(&invocation)
            .ok_or_else(|| Error::Invalid(NOT_A_WORKER_REQUEST.into()))?;
        let response = Response::default();
        let started = std::time::Instant::now();
        let outcome = self.worker.run(&request, Box::new(response.clone()));
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
