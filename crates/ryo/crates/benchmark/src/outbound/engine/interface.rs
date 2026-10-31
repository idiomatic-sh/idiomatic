//! Execute the candidate through the public SDK.
use crate::domain::diagnostics::{errors::Error, types::RunReport};
use crate::domain::suite::types::WorkerRequest;
pub trait EnginePort {
    fn run(&self, request: WorkerRequest) -> Result<RunReport, Error>;
}
