//! The `analysis` port's failures.

use crate::domain::applications::types::LintRuleId;

/// Why a lint call failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AnalysisError {
    /// One rule could not run: its query, or a param it reads that nothing binds.
    #[error("rule `{}` cannot run: {reason}", rule.0)]
    Rule {
        /// The rule.
        rule: LintRuleId,
        /// Why.
        reason: String,
    },
    /// The engine failed.
    #[error("the analysis failed: {reason}")]
    Engine {
        /// What it said.
        reason: String,
    },
}
