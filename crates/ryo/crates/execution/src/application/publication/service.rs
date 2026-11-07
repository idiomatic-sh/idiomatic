//! Invoke the normalization semantics of a producer's publication: every producer-local name becomes a portable one.
use super::errors::PublicationError;
use super::types::PublicationInput;
use crate::domain::finish::publication::escape_cross_publication;

/// Normalizes the names of a producer's table in the output storage the caller owns.
///
/// # Errors
///
/// The reason the producer cannot publish its local names as portable values.
pub fn publication(input: PublicationInput<'_>) -> Result<(), PublicationError> {
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
