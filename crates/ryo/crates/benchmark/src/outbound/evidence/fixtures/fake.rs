//! The fake of the `evidence` port: publication acceptance without external effects.
use super::super::interface::EvidencePort;
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    measurements::types::sample::value::BenchmarkSample,
};

/// A deterministic publication stand-in for application tests.
#[derive(Debug, Clone, Copy)]
pub struct EvidenceFake;
impl EvidencePort for EvidenceFake {
    fn publish(&self, samples: &[BenchmarkSample]) -> Result<(), Error> {
        if samples.is_empty()
            || samples.iter().any(|sample| {
                sample.eligibility
                    != crate::domain::measurements::types::MeasurementEligibility::CompleteInventory
                    || sample.name.0.is_empty()
                    || matches!(
                        sample.measurement.resources,
                        crate::domain::measurements::types::ResourceObservation::WallOnly
                    )
            })
        {
            return Err(Error::Invalid("incomplete evidence fixture".into()));
        }
        Ok(())
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
