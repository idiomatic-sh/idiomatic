//! The `sarif` port's failures.

/// Why a log could not be read.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SarifError {
    /// The log declares that its analysis did not complete successfully.
    #[error("SARIF tool `{tool}` reported an unsuccessful invocation")]
    Incomplete {
        /// The tool whose invocation failed.
        tool: String,
    },
    /// It is not a SARIF log: no output at all is not one either.
    #[error("the output is not a SARIF log: {reason}")]
    Unparsable {
        /// What the parser said.
        reason: String,
    },
    /// A result's location cannot be resolved to a local file.
    #[error("a SARIF result of `{rule}` cannot be located: {reason}")]
    Unlocatable {
        /// The rule.
        rule: String,
        /// Why.
        reason: String,
    },
}
