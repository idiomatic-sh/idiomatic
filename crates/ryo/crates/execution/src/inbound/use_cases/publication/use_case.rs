//! Invoke the existing normalization semantics through the public boundary.
use super::{
    interface::{PublicationError, PublicationUseCase},
    types::PublicationInput,
};
use crate::domain::finish::publication::escape_cross_publication;

pub struct PublicationUseCaseImpl;
impl PublicationUseCase for PublicationUseCaseImpl {
    fn normalize(&self, input: PublicationInput<'_>) -> Result<(), PublicationError> {
        escape_cross_publication(
            input.table,
            input.symbols,
            input.inherited,
            input.worker,
            input.arena,
            input.places,
        )
    }
}
