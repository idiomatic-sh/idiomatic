//! Child-process execution contract.
use ryo_benchmark_contracts::domain::measurements::types::ExecutionProfile;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
/// Execute a prepared invocation and observe its process result.
pub trait ExecutionPort {
    /// Whether numeric exits are directly observed with no synthetic timeout status.
    /// Wrapped, bounded and unknown observers cannot certify process failures.
    fn observes_completed_processes_without_deadline(&self) -> bool {
        false
    }

    /// Exact execution mode and immutable instrumentation settings, without IO.
    fn profile(&self) -> ExecutionProfile;
    /// Launch the child and capture its exit status, streams and elapsed wall time.
    ///
    /// # Errors
    ///
    /// Returns an error when launching or observing the process fails.
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error>;
}
