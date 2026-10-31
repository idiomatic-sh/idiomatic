//! Asymptote owns the ratchet; this port publishes only observed samples.
use crate::domain::{diagnostics::errors::Error, measurements::types::Sample};

/// Publication boundary for complete, correctness-gated observations.
pub trait EvidencePort {
    /// Write one complete document, refusing empty or resource-incomplete samples.
    ///
    /// # Errors
    /// Returns encoding, validation, stream-write or flush failures.
    fn publish(&self, samples: &[Sample]) -> Result<(), Error>;
}
