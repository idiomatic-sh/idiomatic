//! Atomic document encoding precedes writing to the injected destination.
use super::{connection::EvidenceBmfConnection, constants, schemas};
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    measurements::types::sample::value::BenchmarkSample,
};
use crate::outbound::evidence::interface::EvidencePort;
use std::io::Write as _;

#[derive(Debug)]
/// Serializes a complete validated evidence document before publishing any bytes.
pub(crate) struct EvidenceBmfAdapter {
    /// Injected destination and synchronization lifetime.
    connection: EvidenceBmfConnection,
}
impl EvidenceBmfAdapter {
    /// Bind the output connection supplied by executable composition.
    pub(crate) fn new(connection: EvidenceBmfConnection) -> Self {
        Self { connection }
    }
}
impl EvidencePort for EvidenceBmfAdapter {
    fn publish(&self, samples: &[BenchmarkSample]) -> Result<(), Error> {
        let bytes = schemas::encode(samples)?;
        let mut output = self
            .connection
            .output
            .lock()
            .map_err(|_poisoned| Error::Invalid(constants::POISONED.into()))?;
        output.write_all(&bytes)?;
        output.write_all(constants::NEWLINE)?;
        output.flush()?;
        Ok(())
    }
}
