//! Asymptote owns the ratchet; this port publishes only observed samples.
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error, measurements::types::BenchmarkSample,
};

/// Publication boundary for complete, correctness-gated observations.
pub trait EvidencePort {
    /// Write one complete document, refusing empty or resource-incomplete samples.
    ///
    /// # Errors
    /// Returns encoding, validation, stream-write or flush failures.
    fn publish(&self, samples: &[BenchmarkSample]) -> Result<(), Error>;
}
