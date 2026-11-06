//! Invalid project preparation cannot be submitted to the engine.
use super::types::Occurrence;
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum PreparationError {
    #[error("project occurrence {0:?} was not prepared")]
    Missing(Occurrence),
    #[cfg(test)]
    #[error("project occurrence {0:?} was prepared twice")]
    Repeated(Occurrence),
    #[cfg(test)]
    #[error("project occurrence belongs to another declared batch")]
    Foreign,
}
