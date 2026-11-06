//! Refusals owned by rule compilation before file evaluation can begin.
use ryo_values::domain::program::types::RefusedQuery;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;

/// A malformed program, rule identity, or parameter binding.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RuleCompilationError {
    /// Named-root dependency configuration is rejected before any file is read.
    #[error(transparent)]
    Observations(#[from] crate::domain::file_program::errors::FileProgramError),
    /// Rule identities and request bindings retain their domain refusal.
    #[error(transparent)]
    Rules(#[from] RuleRequestsError),
    /// Query compilation preserves its owning rule and exact source diagnostic.
    #[error("rule `{id}`: {error}")]
    Query {
        /// Owning rule ID.
        id: String,
        /// Typed compiler diagnostic with original source provenance.
        error: RefusedQuery,
    },
}
