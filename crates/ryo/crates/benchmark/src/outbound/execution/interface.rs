//! Child-process execution contract.
use crate::domain::diagnostics::errors::DiagnosticsError as Error;
use crate::domain::measurements::types::{Invocation, ProcessOutput};
/// Execute a prepared invocation and observe its process result.
pub trait ExecutionPort {
    /// Launch the child and capture its exit status, streams and elapsed wall time.
    ///
    /// # Errors
    ///
    /// Returns an error when launching or observing the process fails.
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error>;
}
