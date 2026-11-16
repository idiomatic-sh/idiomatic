//! Plain data of the execution port.

/// How a port implementation observes the numeric exit of a completed child.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitObservationMode {
    /// Numeric exits are directly observed with no synthetic timeout status.
    Direct,
    /// Wrapped, bounded and unknown observers cannot certify process failures.
    Mediated,
}

/// The exit of a completed child, decoded from the operating system status.
pub(crate) mod process_exit;

/// Where a worker's request is stored, which the in-process worker reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredRequest(pub std::path::PathBuf);

/// The bytes a worker answers a request with, as a worker child prints them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkerResponse(pub Vec<u8>);

/// The compile time of each rule a stored request names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleCompileTimes(pub std::collections::BTreeMap<ryo::linter::RuleId, std::time::Duration>);

/// The language's worker, run in this process in place of a child: the bench composes it over its engine.
pub trait InProcessWorker: Send + Sync + std::fmt::Debug {
    /// Run the request stored at `request`, answering the response a worker child would print.
    ///
    /// # Errors
    ///
    /// Returns an error when the request cannot be read, run or answered.
    fn run(
        &self,
        request: &StoredRequest,
    ) -> Result<WorkerResponse, ryo_benchmark_contracts::domain::reports::errors::ReportsError>;
    /// Compile the rules of the request stored at `request` ahead of its first run, answering each rule's time.
    ///
    /// # Errors
    ///
    /// Returns an error when the request cannot be read.
    fn compile(&self, request: &StoredRequest) -> Result<RuleCompileTimes, ryo_benchmark_contracts::domain::reports::errors::ReportsError>;
}
