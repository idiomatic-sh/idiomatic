//! The `pattern_matching` port's failures.
use crate::domain::targets::types::Pattern;
use crate::domain::workspaces::types::Reason;

/// Why an operation of `pattern_matching` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum PatternMatchingError {
    /// The pattern is not one.
    #[error("`{pattern}` is not a pattern: {reason}")]
    Invalid {
        /// The pattern.
        pattern: Pattern,
        /// What the matcher said.
        reason: Reason,
    },
}
