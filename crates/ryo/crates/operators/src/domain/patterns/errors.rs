//! The errors of the `matching` concept.

/// Why a pattern could not be searched for: it is no regular expression.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PatternsError {
    /// A query supplied an invalid regular expression.
    #[error("invalid regular expression: {reason}")]
    InvalidRegex {
        /// The regular expression compiler's explanation.
        reason: String,
    },
}
