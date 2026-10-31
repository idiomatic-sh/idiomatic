//! The child process receives a typed request and publishes a typed report.
use crate::domain::diagnostics::{errors::DiagnosticsError as Error, types::RunReport};
use crate::domain::suite::types::WorkerRequest;

/// The worker's process-boundary transport, independently injected from its engine.
pub trait WorkerPort {
    /// Decode the request supplied to this child process.
    ///
    /// # Errors
    /// Returns a failure when the supplied message cannot represent a worker request.
    fn receive(&self) -> Result<WorkerRequest, Error>;

    /// Publish one complete report and flush the response stream.
    ///
    /// # Errors
    /// Returns a failure when encoding, writing or flushing the report fails.
    fn respond(&self, report: &RunReport) -> Result<(), Error>;
}
