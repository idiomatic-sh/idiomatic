//! The certification fake of the execution port: it records launches and lets a scenario change the collection profile on its own.
#![expect(
    clippy::expect_used,
    reason = "fake state is a test precondition; a panic fails the unit or acceptance proof that calls this module"
)]
use super::super::constants;
use ryo_benchmark_contracts::domain::measurements::types::ExecutionProfile;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

#[derive(Debug)]
/// Records launches while allowing independent collection-profile mutation.
pub(super) struct Observations(
    pub(super) crate::outbound::execution::fixtures::fake::ExecutionFake,
    pub(super) std::sync::Mutex<ExecutionProfile>,
);
impl crate::outbound::execution::interface::ExecutionPort for Observations {
    fn profile(&self) -> ExecutionProfile {
        self.1
            .lock()
            .expect(constants::FIXTURE_PRECONDITION)
            .clone()
    }
    fn run(&self, mut invocation: Invocation) -> Result<ProcessOutput, Error> {
        invocation.executable = constants::FIXTURE_FAKE_PROGRAM.into();
        let mut output =
            crate::outbound::execution::interface::ExecutionPort::run(&self.0, invocation)?;
        output.resources =
            ryo_benchmark_contracts::domain::measurements::types::ResourceObservation::Collected(
                ryo_benchmark_contracts::domain::measurements::types::Resources {
                    cpu: std::time::Duration::from_millis(1),
                    peak_rss_bytes:
                        ryo_benchmark_contracts::domain::measurements::types::ResidentBytes(4096),
                },
            );
        Ok(output)
    }
}
