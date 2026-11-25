//! Child-process execution contract.
use super::types::ExitObservationMode;
use benchmark_contracts::domain::measurements::types::ExecutionProfile;
use benchmark_contracts::domain::measurements::types::Invocation;
use benchmark_contracts::domain::measurements::types::ProcessOutput;
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
/// Execute a prepared invocation and observe its process result.
pub trait ExecutionPort: Send + Sync {
    /// Whether numeric exits are directly observed with no synthetic timeout status.
    /// Wrapped, bounded and unknown observers cannot certify process failures.
    fn exit_observation(&self) -> ExitObservationMode {
        ExitObservationMode::Mediated
    }

    /// Exact execution mode and immutable instrumentation settings, without IO.
    fn profile(&self) -> ExecutionProfile;
    /// Compile ahead of its first run the rules `invocation` would run, answering the time each rule took; none when this
    /// execution runs each invocation in a child, which compiles inside the child.
    /// A refusal to compile the rules of a request run in this process is answered, never dropped.
    fn compile(
        &self,
        _invocation: &Invocation,
    ) -> Option<Result<std::collections::BTreeMap<ryo::linter::RuleId, std::time::Duration>, Error>>
    {
        None
    }
    /// Launch the child and capture its exit status, streams and elapsed wall time.
    ///
    /// # Errors
    ///
    /// Returns an error when launching or observing the process fails.
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error>;
}
