//! The errors of the `comparing` concept.

use crate::domain::patterns::errors::PatternsError;

/// Why a comparison could not be decided over a row; the engine maps each 1:1 to its `OperatingError` variant, so the text is unchanged.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ComparingError {
    /// The test is not one of the family's.
    #[error("the test is not one of this family's")]
    WrongTest,
    /// A parameter the comparison reads has no binding.
    #[error("unbound parameter")]
    Unbound,
    /// A bound value does not fit the comparison.
    #[error("wrong binding")]
    WrongBinding,
    /// A cell the comparison reads is not in the row.
    #[error(transparent)]
    Rows(#[from] ryo_values::domain::rows::errors::RowsError),
    /// A query supplied an invalid regular expression.
    #[error("invalid regular expression: {reason}")]
    InvalidRegex {
        /// The regular expression compiler's explanation.
        reason: String,
    },
}

impl From<PatternsError> for ComparingError {
    /// A search's fault is the comparison's, variant for variant.
    fn from(error: PatternsError) -> Self {
        match error {
            PatternsError::InvalidRegex { reason } => Self::InvalidRegex { reason },
        }
    }
}
