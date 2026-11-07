//! What the `prepare` use case promises its callers, in the core's own vocabulary.
use super::types::PrepareRequest;
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use crate::domain::operating::errors::OperatingError;

// Canonical inbound signature vocabulary from private pure concepts.
pub use crate::domain::prepared_request::types::ObservationIdentity;

/// What `prepare` offers its callers.
pub trait PrepareUseCase {
    /// Prepares the program of a request: authored plans are assembled, compiled queries are linked.
    ///
    /// # Errors
    ///
    /// Refuses a request whose plans, bindings and rules do not agree.
    fn prepare(&self, request: PrepareRequest) -> Result<PreparedExecutionProgram, OperatingError>;
}
