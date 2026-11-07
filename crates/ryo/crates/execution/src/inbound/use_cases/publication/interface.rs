//! Normalize producer-local names before the producer's symbols are dropped.
use super::errors::PublicationError;
use super::types::PublicationInput;
pub use crate::domain::finish::types::PublicationReason;

pub trait PublicationUseCase: std::fmt::Debug + Send + Sync {
    fn publication(&self, input: PublicationInput<'_>) -> Result<(), PublicationError>;
}
