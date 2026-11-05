//! Asymptote owns the ratchet; this port publishes only observed samples.
use crate::outbound::evidence::types::publication::value::Publication;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

/// Publication boundary for complete, correctness-gated observations.
pub trait EvidencePort {
    /// Write one complete document, refusing empty or resource-incomplete samples.
    ///
    /// # Errors
    /// Returns encoding, validation, stream-write or flush failures.
    fn publish(&self, publication: &Publication<'_>) -> Result<(), Error>;
}
