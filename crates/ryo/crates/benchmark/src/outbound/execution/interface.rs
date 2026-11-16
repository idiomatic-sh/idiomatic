//! Child-process execution contract.
use super::types::ExitObservationMode;
use ryo_benchmark_contracts::domain::measurements::types::ExecutionProfile;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
/// Execute a prepared invocation and observe its process result.
pub trait ExecutionPort: Send + Sync {
    /// Whether numeric exits are directly observed with no synthetic timeout status.
    /// Wrapped, bounded and unknown observers cannot certify process failures.
    fn exit_observation(&self) -> ExitObservationMode {
        ExitObservationMode::Mediated
    }

    /// Exact execution mode and immutable instrumentation settings, without IO.
    fn profile(&self) -> ExecutionProfile;
    /// Compile ahead of its first run the rules `invocation` would run, answering the time each rule took. An execution that
    /// runs each invocation in a child compiles inside the child, so it answers none.
    ///
    /// # Errors
    ///
    /// Returns an error when the invocation's request cannot be read or its rules cannot be loaded.
    fn compile(
        &self,
        _invocation: &Invocation,
    ) -> Result<std::collections::BTreeMap<ryo::linter::RuleId, std::time::Duration>, Error> {
        Ok(std::collections::BTreeMap::new())
    }
    /// Launch the child and capture its exit status, streams and elapsed wall time.
    ///
    /// # Errors
    ///
    /// Returns an error when launching or observing the process fails.
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error>;
}
