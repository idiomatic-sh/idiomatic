//! The fake of the `evidence` port: publication acceptance without external effects.
use super::super::interface::EvidencePort;
use crate::domain::{diagnostics::errors::Error, measurements::types::Sample};

/// A deterministic publication stand-in for application tests.
#[derive(Debug, Clone, Copy)]
pub struct EvidenceFake;
impl EvidencePort for EvidenceFake {
    fn publish(&self, samples: &[Sample]) -> Result<(), Error> {
        if samples.is_empty()
            || samples
                .iter()
                .any(|sample| sample.name.is_empty() || sample.measurement.resources.is_none())
        {
            return Err(Error::Invalid("incomplete evidence fixture".into()));
        }
        Ok(())
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
