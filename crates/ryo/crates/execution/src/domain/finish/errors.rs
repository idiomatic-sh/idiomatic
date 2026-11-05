//! A producer publication cannot retain a truthful portable name mapping.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum FinishError {
    #[error("{reason}")]
    Publication { reason: Box<str> },
}
