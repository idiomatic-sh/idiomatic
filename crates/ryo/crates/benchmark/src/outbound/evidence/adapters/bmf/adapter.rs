//! Atomic document encoding precedes writing to the injected destination.
use super::{connection::EvidenceBmfConnection, constants, schemas};
use crate::domain::{diagnostics::errors::Error, measurements::types::Sample};
use crate::outbound::evidence::interface::EvidencePort;
use std::io::Write;

#[derive(Debug)]
pub(crate) struct EvidenceBmfAdapter {
    connection: EvidenceBmfConnection,
}
impl EvidenceBmfAdapter {
    pub(crate) fn new(connection: EvidenceBmfConnection) -> Self {
        Self { connection }
    }
}
impl EvidencePort for EvidenceBmfAdapter {
    fn publish(&self, samples: &[Sample]) -> Result<(), Error> {
        let bytes = schemas::encode(samples)?;
        let mut output = self
            .connection
            .output
            .lock()
            .map_err(|_| Error::Invalid(constants::POISONED.into()))?;
        output.write_all(&bytes)?;
        output.write_all(constants::NEWLINE)?;
        output.flush()?;
        Ok(())
    }
}
