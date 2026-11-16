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
    ) -> Result<(), Error> {
        Ok(())
    }
    /// An empty set of costs is refused; any other goes nowhere.
    fn costs(
        &self,
        _language: &ryo_benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage,
        costs: &ryo_benchmark_contracts::domain::costs::types::ProjectCosts,
    ) -> Result<(), Error> {
        if costs.0.is_empty() {
            return Err(Error::Invalid("no costs".into()));
        }
        Ok(())
    }
    /// The fake has no external effects, so the verification times go nowhere.
    fn verification(
        &self,
        _language: &ryo_benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage,
        _timings: &crate::outbound::evidence::types::RuleTimings,
    ) -> Result<(), Error> {
        Ok(())
    }
    /// The fake has no external effects, so the growth goes nowhere.
    fn growth(
        &self,
        _language: &ryo_benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage,
        _growth: &ryo_benchmark_contracts::domain::measurements::types::ShapeGrowths,
    ) -> Result<(), Error> {
        Ok(())
    }
    /// The fake reads no file: a rule's file is refused, and no file gives no times.
    fn read_verification(
        &self,
        files: &crate::outbound::evidence::types::VerificationTimesFiles,
    ) -> Result<crate::outbound::evidence::types::RuleTimings, Error> {
        if files.0.is_empty() {
            Ok(crate::outbound::evidence::types::RuleTimings::default())
        } else {
            Err(Error::Invalid(
                "the fake reads no verification times".into(),
            ))
        }
    }
    /// The fake writes nothing.
    fn finish(&self) -> Result<(), Error> {
        Ok(())
    }
    /// The fake has no external effects, so the parity evidence goes nowhere.
    fn parity(
        &self,
        _parity: &ryo_benchmark_contracts::domain::reports::types::ParityReport<'_>,
    ) -> Result<(), Error> {
        Ok(())
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;

/// An evidence port that keeps what it is given: each project's costs, the verification times, the growth and how many
/// documents were finished, for the use cases whose rows read what was published.
#[derive(Debug, Default)]
pub struct EvidenceRecorder {
    /// What was given so far.
    pub recorded: std::sync::Mutex<Recorded>,
}

/// What an [`EvidenceRecorder`] was given.
#[derive(Clone, Debug, Default)]
pub struct Recorded {
    /// Every project's cost, in the order given.
    pub costs: Vec<ryo_benchmark_contracts::domain::costs::types::ProjectCost>,
    /// Every rule's times.
    pub verification: crate::outbound::evidence::types::RuleTimings,
    /// Every shape's growth.
    pub growth: ryo_benchmark_contracts::domain::measurements::types::ShapeGrowths,
    /// How many documents were finished.
    pub documents: usize,
    /// How many comparisons were reported.
    pub comparisons: usize,
}

impl EvidenceRecorder {
    /// What was given so far.
    #[must_use]
    pub fn recorded(&self) -> Recorded {
        self.recorded.lock().map_or_else(
            |poisoned| poisoned.into_inner().clone(),
            |held| held.clone(),
        )
    }

    /// Change what was recorded.
    fn record(&self, change: impl FnOnce(&mut Recorded)) -> Result<(), Error> {
        let mut held = self
            .recorded
            .lock()
            .map_err(|_poisoned| Error::Invalid("the recorder's lock is poisoned".into()))?;
        if held.documents > 0 {
            return Err(Error::Invalid("the document is already written".into()));
        }
        change(&mut held);
        Ok(())
    }
}

impl EvidencePort for EvidenceRecorder {
    fn publish(&self, publication: &MeasuredPublication<'_>) -> Result<(), Error> {
        EvidenceFake.publish(publication)
    }
    fn comparison(
        &self,
        _reference: &RunReport,
        _candidate: &RunReport,
        _comparison: ReportComparison,
        _contract: ComparisonContract,
    ) -> Result<(), Error> {
        self.record(|held| held.comparisons += 1)
    }
    fn costs(
        &self,
        language: &ryo_benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage,
        costs: &ryo_benchmark_contracts::domain::costs::types::ProjectCosts,
    ) -> Result<(), Error> {
        EvidenceFake.costs(language, costs)?;
        self.record(|held| held.costs.extend(costs.0.iter().cloned()))
    }
    fn verification(
        &self,
        _language: &ryo_benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage,
        timings: &crate::outbound::evidence::types::RuleTimings,
    ) -> Result<(), Error> {
        self.record(|held| {
            held.verification.0.extend(
                timings
                    .0
                    .iter()
                    .map(|(rule, timing)| (rule.clone(), *timing)),
            );
        })
    }
    fn growth(
        &self,
        _language: &ryo_benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage,
        growth: &ryo_benchmark_contracts::domain::measurements::types::ShapeGrowths,
    ) -> Result<(), Error> {
        self.record(|held| {
            held.growth.0.extend(
                growth
                    .0
                    .iter()
                    .map(|(shape, grown)| (shape.clone(), *grown)),
            );
        })
    }
    fn read_verification(
        &self,
        files: &crate::outbound::evidence::types::VerificationTimesFiles,
    ) -> Result<crate::outbound::evidence::types::RuleTimings, Error> {
        EvidenceFake.read_verification(files)
    }
    fn finish(&self) -> Result<(), Error> {
        self.record(|held| held.documents += 1)
    }
    fn parity(
        &self,
        parity: &ryo_benchmark_contracts::domain::reports::types::ParityReport<'_>,
    ) -> Result<(), Error> {
        EvidenceFake.parity(parity)
    }
}
