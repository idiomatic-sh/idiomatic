//! What the `prepare` use case promises its callers, in the core's own vocabulary.
use super::types::PrepareRequest;
use crate::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution_domain::domain::operating::errors::OperatingError;

// The compiled queries and the compilation the contract is made of.
pub use crate::application::rule_compilation::errors::RuleCompilationError;
pub use crate::application::rule_compilation::service::CompiledPlans;
pub use crate::application::rule_compilation::service::CompiledQueries;
pub use crate::application::rule_compilation::service::ExecutionCompilation;
pub use crate::application::rule_compilation::service::PreparedQueries;
pub use crate::application::rule_compilation::service::QueryCompilation;
pub use crate::application::rule_compilation::types::PreparedRuleIdentities;
// Canonical inbound signature vocabulary from private pure concepts.
pub use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
// The derived capability a prepared program holds; an edge hands it to the use case, so callers outside the engine crates never name the store.
pub use ryo_execution_store::application::file_dag::derived::owner::types::DerivedAccess;

/// What `prepare` offers its callers.
pub trait PrepareUseCase {
    /// Prepares the program of a request: authored plans are assembled, compiled queries are linked.
    ///
    /// # Errors
    ///
    /// Refuses a request whose plans, bindings and rules do not agree.
    fn prepare(&self, request: PrepareRequest) -> Result<PreparedExecutionProgram, OperatingError>;
}
