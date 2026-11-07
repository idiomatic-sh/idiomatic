use crate::domain::operating::errors::OperatingError;
use crate::outbound::facts::errors::FactsError;

/// Why the frontiers of one rule could not be prepared.
#[derive(Debug, thiserror::Error)]
pub enum PreparationError {
    #[error(transparent)]
    Facts(#[from] FactsError),
    #[error(transparent)]
    Metadata(#[from] OperatingError),
}
