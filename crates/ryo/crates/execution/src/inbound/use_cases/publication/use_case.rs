//! Thin public orchestration over the one private publication owner.
use super::interface::PublicationUseCase;
use crate::application::publication::errors::PublicationError;
use crate::application::publication::service::publication;
use crate::application::publication::types::PublicationInput;

/// Publishes a producer's table; the normalization is the engine's own.
#[derive(Debug, Default)]
pub struct PublicationUseCaseImpl;

impl PublicationUseCaseImpl {
    /// The use case over the engine's publication.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl PublicationUseCase for PublicationUseCaseImpl {
    fn publication(&self, input: PublicationInput<'_>) -> Result<(), PublicationError> {
        publication(input)
    }
}
