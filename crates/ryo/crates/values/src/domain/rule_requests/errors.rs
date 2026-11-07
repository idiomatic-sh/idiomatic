//! The `rules` concept's failures.

use crate::domain::runs::types::RuleId;

/// Why a rule of a run is refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RuleRequestsError {
    /// A bound producer's observation is unavailable in this generation.
    #[error("rule `{}` has incomplete observation input: {reason}", id.0)]
    ObservationIncomplete { id: RuleId, reason: String },
    /// A rule has no id, so no finding could name it.
    #[error("a rule has a blank id")]
    BlankRuleId,
    /// Two rules have the same id, so a finding could not say which gave it.
    #[error("two rules are named `{}`", id.0)]
    DuplicateRuleId {
        /// The id both have.
        id: RuleId,
    },
    /// A rule's query has a parameter nothing was bound to.
    #[error("rule `{}` has no value for a parameter it declares", id.0)]
    MissingParameter {
        /// The rule's id.
        id: RuleId,
    },
    /// This rule asked for a valid semantic answer that its provider cannot supply.
    #[error("rule `{}` has incomplete semantic coverage: {reason}", id.0)]
    SemanticIncomplete {
        /// The consuming rule, even when another rule reused the same demand.
        id: RuleId,
        /// The exact provider or application reason.
        reason: String,
    },
    /// A rule's query cannot be run: an operator or a text step refused it.
    #[error("rule `{}`: {reason}", id.0)]
    Failed {
        /// The rule's id.
        id: RuleId,
        /// Why.
        reason: String,
    },
}
