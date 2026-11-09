//! Atomic document encoding precedes writing to the injected destination.
use super::{connection::EvidenceBmfConnection, constants, schemas};
use crate::outbound::evidence::interface::EvidencePort;
use crate::outbound::evidence::types::measured_publication::value::MeasuredPublication;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{
    ComparisonContract, ReportComparison, RunReport,
};
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
    fn publish(&self, publication: &MeasuredPublication<'_>) -> Result<(), Error> {
        let bytes = schemas::encode(publication)?;
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
    fn comparison(
        &self,
        reference: &RunReport,
        candidate: &RunReport,
        comparison: ReportComparison,
        contract: ComparisonContract,
    ) -> Result<(), Error> {
        let line = schemas::comparison(reference, candidate, comparison, contract)?;
        let mut output = self
            .connection
            .comparison
            .lock()
            .map_err(|_poisoned| Error::Invalid(constants::POISONED.into()))?;
        output.write_all(line.as_bytes())?;
        output.write_all(constants::NEWLINE)?;
        output.flush()?;
        Ok(())
    }
    fn parity(
        &self,
        parity: &ryo_benchmark_contracts::domain::reports::types::ParityReport<'_>,
    ) -> Result<(), Error> {
        let metrics = schemas::parity_metrics(parity)?;
        let rules = schemas::parity_rules(parity)?;
        let mut output = self
            .connection
            .output
            .lock()
            .map_err(|_poisoned| Error::Invalid(constants::POISONED.into()))?;
        output.write_all(&metrics)?;
        output.write_all(constants::NEWLINE)?;
        output.flush()?;
        let mut comparison = self
            .connection
            .comparison
            .lock()
            .map_err(|_poisoned| Error::Invalid(constants::POISONED.into()))?;
        comparison.write_all(&rules)?;
        comparison.write_all(constants::NEWLINE)?;
        comparison.flush()?;
        Ok(())
    }
}
