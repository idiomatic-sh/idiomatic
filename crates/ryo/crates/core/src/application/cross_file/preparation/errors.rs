//! Invalid project preparation cannot be submitted to the engine.
use super::types::OccurrenceNumber;
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum BatchPreparationError {
    #[error("project occurrence {0:?} was not prepared")]
    Missing(OccurrenceNumber),
    #[cfg(test)]
    #[error("project occurrence {0:?} was prepared twice")]
    Repeated(OccurrenceNumber),
    #[cfg(test)]
    #[error("project occurrence belongs to another declared batch")]
    Foreign,
}
