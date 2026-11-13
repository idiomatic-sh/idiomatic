//! A worker standing in for the profiled children: each run answers the next of its given outputs.
use crate::outbound::execution::interface::ExecutionPort;
use ryo_benchmark_contracts::domain::measurements::types::{
    CapturedBytes, ExecutionProfile, Invocation, ProcessExitCode, ProcessOutput,
    ResourceObservation, TerminationSignal,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The profiled children: the heap child first, then the CPU child.
#[derive(Clone, Debug)]
pub struct ChildrenFake {
    /// The standard output of each run in turn; the last answers every later run.
    pub stdout: Vec<CapturedBytes>,
    /// The exit status of every run.
    pub status: ProcessExitCode,
    /// The wall time of every run.
    pub wall: Duration,
    /// The runs received, in order.
    pub history: Arc<Mutex<Vec<Invocation>>>,
}
impl Default for ChildrenFake {
    fn default() -> Self {
        Self {
            stdout: vec![CapturedBytes(b"diagnostics".to_vec())],
            status: ProcessExitCode(0),
            wall: Duration::from_secs(330),
            history: Arc::default(),
        }
    }
}
impl ExecutionPort for ChildrenFake {
    fn profile(&self) -> ExecutionProfile {
        ExecutionProfile::Wall
    }

    #[expect(
        clippy::expect_used,
        reason = "a poisoned fixture lock must fail the test"
    )]
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        let mut history = self.history.lock().expect("fixture history");
        let stdout = self
            .stdout
            .get(history.len())
            .or_else(|| self.stdout.last())
            .cloned()
            .unwrap_or_else(|| CapturedBytes(Vec::new()));
        history.push(invocation);
        Ok(ProcessOutput {
            signal: TerminationSignal::Exited,
            status: self.status,
            stdout,
            stderr: CapturedBytes(Vec::new()),
            wall: self.wall,
            resources: ResourceObservation::WallOnly,
        })
    }
}
