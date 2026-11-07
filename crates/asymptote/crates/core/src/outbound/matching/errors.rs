//! The `matching` port's failures.

/// Why a pattern could not be matched.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MatchingError {
    /// It is no glob.
    #[error("`{pattern}` is no glob: {reason}")]
    Pattern {
        /// The pattern.
        pattern: String,
        /// What the parser said.
        reason: String,
    },
}
