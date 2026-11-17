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

impl From<PatternsError> for crate::domain::contract::types::PortableOperatorFault {
    /// A pattern that is no regular expression, as the run reports it: the matcher's reason, unchanged.
    fn from(error: PatternsError) -> Self {
        let PatternsError::InvalidRegex { reason } = error;
        Self::Pattern(
            ryo_matching::inbound::use_cases::matching::errors::MatchingError::InvalidRegex {
                reason,
            },
        )
    }
}
