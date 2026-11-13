//! The worker's identity is checked on the decoded request, before any engine loads.
use super::types::WorkerExpectation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
/// Check the request the transport delivers against what the executable expects.
pub trait CheckWorkerRequestUseCase {
    /// Receive the request and compare its parser configuration and rule phases.
    ///
    /// # Errors
    /// Refuses an undecodable request and any target or phase identity other than the expected one.
    fn check_worker_request(&self, expectation: &WorkerExpectation) -> Result<(), ReportsError>;
}
