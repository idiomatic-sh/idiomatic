//! Why a registered case's input identities are refused.
use super::types::RefusalText;

/// A refused registered input declaration.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum RegisteredInputsError {
    /// Refuses: metadata names an undeclared source.
    #[error("metadata names an undeclared source")]
    UndeclaredMetadataSource,
    /// Refuses: required input observations are missing.
    #[error("required input observations are missing")]
    InputObservationsMissing,
    /// A library refused the declaration with these words.
    #[error("{}", .0.0)]
    Refused(RefusalText),
}
