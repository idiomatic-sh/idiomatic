//! What the `publication` use case promises its callers, in the core's own vocabulary.

// The borrowed publication data and the refusal the contract is made of.
pub use crate::application::publication::errors::PublicationError;
pub use crate::application::publication::types::PublicationInput;
// Canonical inbound signature vocabulary from private pure concepts.
pub use ryo_execution_domain::domain::finish::types::PublicationReason;

/// What `publication` offers its callers.
pub trait PublicationUseCase: Send + Sync {
    /// Normalizes the names of a producer's table in the output storage the caller owns.
    ///
    /// # Errors
    ///
    /// The reason the producer cannot publish its local names as portable values.
    fn publication(&self, input: PublicationInput<'_>) -> Result<(), PublicationError>;
}
