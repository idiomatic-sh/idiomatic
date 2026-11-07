//! The fake of the `evidence` port: publication acceptance without external effects.
use super::super::interface::EvidencePort;
use crate::outbound::evidence::types::measured_publication::value::MeasuredPublication;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{
    ComparisonContract, ReportComparison, RunReport,
};

/// A deterministic publication stand-in for application tests.
#[derive(Debug, Clone, Copy)]
pub struct EvidenceFake;
impl EvidencePort for EvidenceFake {
    fn publish(&self, publication: &MeasuredPublication<'_>) -> Result<(), Error> {
        let samples = publication.samples();
        if samples.is_empty()
            || samples.iter().any(|sample| {
                sample.inventory != ryo_benchmark_contracts::domain::measurements::types::InventoryStatus::Complete
                    || sample.name.0.is_empty()
                    || matches!(
                        sample.measurement.resources,
                        ryo_benchmark_contracts::domain::measurements::types::ResourceObservation::WallOnly
                    )
            })
        {
            return Err(Error::Invalid("incomplete evidence fixture".into()));
        }
        Ok(())
    }
    /// The fake has no external effects, so the comparison evidence goes nowhere.
    fn comparison(
        &self,
        _reference: &RunReport,
        _candidate: &RunReport,
        _comparison: ReportComparison,
        _contract: ComparisonContract,
    ) {
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
