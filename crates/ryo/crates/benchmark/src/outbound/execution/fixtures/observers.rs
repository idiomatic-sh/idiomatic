//! Execution substitutes that observe fixed byte streams, or declare an observer with no deadline.
use super::fake::ExecutionFake;
use crate::outbound::execution::interface::ExecutionPort;
use ryo_benchmark_contracts::domain::measurements::types::{
    CapturedBytes, ExecutionProfile, Invocation, ProcessExitCode, ProcessOutput,
    ResourceObservation, TerminationSignal,
};

/// Different observed byte streams, independent of any certificate matching implementation.
pub struct Streams {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
impl ExecutionPort for Streams {
    fn profile(&self) -> ExecutionProfile {
        ExecutionProfile::Wall
    }
    fn exit_observation(&self) -> crate::outbound::execution::types::ExitObservationMode {
        crate::outbound::execution::types::ExitObservationMode::Direct
    }
    fn run(
        &self,
        _: Invocation,
    ) -> Result<ProcessOutput, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
        Ok(ProcessOutput {
            status: ProcessExitCode(2),
            signal: TerminationSignal::Exited,
            stdout: CapturedBytes(self.stdout.clone()),
            stderr: CapturedBytes(self.stderr.clone()),
            wall: std::time::Duration::ZERO,
            resources: ResourceObservation::WallOnly,
        })
    }
}

/// Explicit no-deadline capability; the underlying fixture retains actual invocation history.
pub struct Observer(pub ExecutionFake);
impl ExecutionPort for Observer {
    fn profile(&self) -> ExecutionProfile {
        self.0.profile()
    }
    fn exit_observation(&self) -> crate::outbound::execution::types::ExitObservationMode {
        crate::outbound::execution::types::ExitObservationMode::Direct
    }
    fn run(
        &self,
        invocation: Invocation,
    ) -> Result<ProcessOutput, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
        self.0.run(invocation)
    }
}
