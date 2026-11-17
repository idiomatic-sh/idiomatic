//! Public refusal to publish producer-local names as portable values.
use ryo_execution_domain::domain::finish::types::PublicationReason;

/// Why a table cannot be published.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PublicationError {
    /// A producer-local name cannot become a portable value.
    #[error("{reason}")]
    Publication {
        /// Why the publication was refused.
        reason: PublicationReason,
    },
}
