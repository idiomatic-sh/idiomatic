//! Execute the worker using typed engine and process-transport boundaries.
use crate::domain::diagnostics::errors::Error;
/// One child-process request, candidate evaluation and flushed response.
pub trait RunRyoUseCase {
    /// Receive, evaluate and publish one report in that order.
    ///
    /// # Errors
    /// Propagates receive, engine and response failures without publishing a clean result.
    fn run_ryo(&self) -> Result<(), Error>;
}
