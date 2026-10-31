//! Execute the LinterRyoAdapter child worker through the injected engine.
use crate::domain::diagnostics::{errors::Error, types::RunReport};
use crate::domain::suite::types::WorkerRequest;
pub trait RunRyoUseCase {
    fn run_ryo(&self, request: WorkerRequest) -> Result<RunReport, Error>;
}
