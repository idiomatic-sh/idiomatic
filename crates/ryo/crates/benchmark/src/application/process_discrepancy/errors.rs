//! Why an actual reference process does not match its declared failure.
use crate::domain::registered_inputs::types::RefusalText;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;

/// A refused actual reference process.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ProcessDiscrepancyError {
    /// Refuses: the case declares no process expectation.
    #[error("no process expectation")]
    NoProcessExpectation,
    /// Refuses: the actual process is not the validated exact observation group.
    #[error("actual process identity is not the validated exact observation group")]
    IdentityNotValidatedGroup,
    /// Refuses: the termination differs from the declared completed failure.
    #[error("reference termination differs from declared completed failure")]
    TerminationDiffers,
    /// Refuses: the bytes differ from the declared expectation.
    #[error("reference process bytes differ from declared expectation")]
    BytesDiffer,
    /// Refuses: the rules directory names no profile.
    #[error("missing process profile boundary")]
    MissingProfileBoundary,
    /// A library or a boundary refused with these words.
    #[error("{}", .0.0)]
    Refused(RefusalText),
}

impl From<String> for ProcessDiscrepancyError {
    fn from(text: String) -> Self {
        Self::Refused(RefusalText(text))
    }
}

impl From<ProcessDiscrepancyError> for ReportsError {
    fn from(error: ProcessDiscrepancyError) -> Self {
        Self::Invalid(error.to_string())
    }
}
