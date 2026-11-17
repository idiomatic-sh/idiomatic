//! A worker's request runs in this process through the language's worker; the composition chooses this execution for the
//! candidate, and any invocation that is no worker request is refused.
use super::constants::{COMPLETED, FAILED, NOT_A_WORKER_REQUEST};
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::execution::types::{InProcessWorker, StoredRequest};
use ryo_benchmark_contracts::domain::measurements::types::{
    CapturedBytes, ExecutionProfile, Invocation, ProcessExitCode, ProcessOutput,
    ResourceObservation, TerminationSignal,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::constants::RUN_RYO;

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
impl ExecutionPort for ExecutionInProcessAdapter<'_> {
    fn profile(&self) -> ExecutionProfile {
        ExecutionProfile::Wall
    }
    fn compile(
        &self,
        invocation: &Invocation,
    ) -> Option<Result<std::collections::BTreeMap<ryo::linter::RuleId, std::time::Duration>, Error>> {
        // An invocation that is no worker request runs as a child, which compiles inside it.
        let request = Self::request_file(invocation)?;
        Some(self.worker.compile(&request).map(|times| times.0))
    }
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        let request = Self::request_file(&invocation)
            .ok_or_else(|| Error::Invalid(NOT_A_WORKER_REQUEST.into()))?;
        let started = std::time::Instant::now();
        let outcome = self.worker.run(&request);
        let wall = started.elapsed();
        let (status, stdout, stderr) = match outcome {
            Ok(response) => (COMPLETED, response.0, Vec::new()),
            Err(error) => (FAILED, Vec::new(), error.to_string().into_bytes()),
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
