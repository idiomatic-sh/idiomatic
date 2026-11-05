//! The `matching` port's failures.

/// Why an operation of `matching` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MatchingError {
    /// A glob is not one.
    #[error("`{glob}` is not a glob: {reason}")]
    BadGlob {
        /// The glob as written.
        glob: String,
        /// What was wrong with it.
        reason: String,
    },
}
