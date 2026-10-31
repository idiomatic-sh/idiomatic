//! Execute the LinterRyoAdapter child worker through the injected engine.
use crate::domain::diagnostics::{errors::Error, types::RunReport};
use crate::domain::suite::types::WorkerRequest;
/// Run the candidate worker through its injected SDK boundary.
pub trait RunRyoUseCase {
    /// Forward a worker request and preserve its report or failure.
    ///
    /// # Errors
    ///
    /// Returns the engine boundary failure when candidate evaluation cannot complete.
    fn run_ryo(&self, request: WorkerRequest) -> Result<RunReport, Error>;
}
