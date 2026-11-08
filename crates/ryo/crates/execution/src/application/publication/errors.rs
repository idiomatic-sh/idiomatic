//! Public refusal to publish producer-local names as portable values.
use ryo_execution_domain::domain::finish::types::PublicationReason;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PublicationError {
    #[error("{reason}")]
    Publication { reason: PublicationReason },
}
