//! The child process receives a typed request and publishes a typed report.
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;

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

    /// Decode the request stored at `path`, as `receive` decodes the one supplied to this process: a worker run in the bench's
    /// own process reads each request it is handed this way.
    ///
    /// # Errors
    /// Returns a failure when the file cannot be read or does not represent a worker request.
    fn read(&self, path: &std::path::Path) -> Result<WorkerRequest, Error>;

    /// The bytes `respond` publishes for `report`, framing included, without publishing them.
    ///
    /// # Errors
    /// Returns a failure when the report cannot be encoded.
    fn encoded(&self, report: &RunReport) -> Result<super::types::EncodedResponse, Error>;
}
