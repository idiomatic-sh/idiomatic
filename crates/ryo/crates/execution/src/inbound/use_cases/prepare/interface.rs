//! What the `prepare` use case promises its callers, in the core's own vocabulary.
use super::types::PrepareRequest;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;

// Canonical inbound signature vocabulary from private pure concepts.
pub use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
// The derived capability a prepared program holds; an edge hands it to the use case, so callers outside the engine crates never name the store.
pub use ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess;

/// What `prepare` offers its callers.
pub trait PrepareUseCase {
    /// Prepares the program of a request: authored plans are assembled, rules `compile` compiled are linked.
    ///
    /// # Errors
    ///
    /// Refuses a request whose plans, bindings and rules do not agree.
    fn prepare(&self, request: PrepareRequest) -> Result<PreparedExecutionProgram, OperatingError>;
}
