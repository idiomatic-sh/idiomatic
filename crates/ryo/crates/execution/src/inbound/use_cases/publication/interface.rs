//! Normalize producer-local names before the producer's symbols are dropped.
use super::types::PublicationInput;
pub use crate::domain::finish::errors::FinishError as PublicationError;

pub trait PublicationUseCase {
    fn normalize(&self, input: PublicationInput<'_>) -> Result<(), PublicationError>;
}
