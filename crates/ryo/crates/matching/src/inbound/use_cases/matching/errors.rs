//! Semantic compilation refusals, independent of the execution engine.

/// Why matching could not compile an authored expression.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MatchingError {
    /// The regex syntax is invalid.
    #[error("invalid regex: {reason}")]
    InvalidRegex {
        /// The compiler's reason.
        reason: String,
    },
    /// The glob syntax is invalid.
    #[error("`{glob}` is not a glob: {reason}")]
    BadGlob {
        /// The authored glob, or empty for a set compilation error.
        glob: String,
        /// The compiler's reason.
        reason: String,
    },
}
