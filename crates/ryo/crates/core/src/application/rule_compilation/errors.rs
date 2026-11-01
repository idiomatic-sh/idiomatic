//! Refusals owned by rule compilation before file evaluation can begin.
use crate::domain::rules::errors::RulesError;
use ryo_values::domain::plan_parts::types::RefusedQuery;

/// A malformed program, rule identity, or parameter binding.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum RuleCompilationError {
    /// Rule identities and request bindings retain their domain refusal.
    #[error(transparent)]
    Rules(#[from] RulesError),
    /// Query compilation preserves its owning rule and exact source diagnostic.
    #[error("rule `{id}`: {error}")]
    Query {
        /// Owning rule ID.
        id: String,
        /// Typed compiler diagnostic with original source provenance.
        error: RefusedQuery,
    },
}
