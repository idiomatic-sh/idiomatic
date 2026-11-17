//! Atomic document encoding precedes writing to the injected destination.
use super::{connection::EvidenceBmfConnection, constants, schemas};
use crate::outbound::evidence::interface::EvidencePort;
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
impl EvidenceBmfAdapter {
    /// Add measures to the document not yet written.
    fn add(
        &self,
        adding: impl FnOnce(&mut super::types::MeasureDocument) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut held = self
            .connection
            .document
            .lock()
            .map_err(|_poisoned| Error::Invalid(constants::POISONED.into()))?;
        adding(
            held.as_mut()
                .ok_or_else(|| Error::Invalid(constants::FINISHED.into()))?,
        )
    }
}

impl EvidencePort for EvidenceBmfAdapter {
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
    fn costs(
        &self,
        language: &ryo_benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage,
        costs: &ryo_benchmark_contracts::domain::costs::types::ProjectCosts,
    ) -> Result<(), Error> {
        self.add(|held| schemas::costs(held, language, costs))
    }
    fn verification(
        &self,
        language: &ryo_benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage,
        timings: &crate::outbound::evidence::types::RuleTimings,
    ) -> Result<(), Error> {
        self.add(|held| schemas::verification(held, language, timings))
    }
    fn growth(
        &self,
        language: &ryo_benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage,
        growth: &ryo_benchmark_contracts::domain::measurements::types::ShapeGrowths,
    ) -> Result<(), Error> {
        self.add(|held| schemas::growth(held, language, growth))
    }
    fn read_verification(
        &self,
        files: &crate::outbound::evidence::types::VerificationTimesFiles,
    ) -> Result<crate::outbound::evidence::types::RuleTimings, Error> {
        schemas::read_verification(files)
    }
    fn finish(&self) -> Result<(), Error> {
        let mut held = self
            .connection
            .document
            .lock()
            .map_err(|_poisoned| Error::Invalid(constants::POISONED.into()))?;
        let bytes = schemas::encode(
            held.as_ref()
                .ok_or_else(|| Error::Invalid(constants::FINISHED.into()))?,
        )?;
        let mut written = bytes;
        written.extend_from_slice(constants::NEWLINE);
        self.connection
            .write_document(&super::types::EncodedDocument(written))?;
        *held = None;
        Ok(())
    }
}
