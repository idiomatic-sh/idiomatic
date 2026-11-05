//! Invoke the existing normalization semantics through the public boundary.
use super::{errors::PublicationError, interface::PublicationUseCase, types::PublicationInput};
use crate::domain::finish::publication::escape_cross_publication;

#[derive(Debug)]
pub struct PublicationUseCaseImpl;
impl PublicationUseCaseImpl {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}
impl PublicationUseCase for PublicationUseCaseImpl {
    fn publication(&self, input: PublicationInput<'_>) -> Result<(), PublicationError> {
        escape_cross_publication(
            input.table,
            input.symbols,
            input.inherited,
            input.worker,
            input.arena,
            input.places,
        )
        .map_err(|reason| PublicationError::Publication { reason })
    }
}
