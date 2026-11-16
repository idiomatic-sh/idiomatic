//! Asymptote owns the ratchet; this port gathers one document of observed measures and writes it once.
use crate::outbound::evidence::types::measured_publication::value::MeasuredPublication;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::{
    ComparisonContract, ReportComparison, RunReport,
};

/// Publication boundary for complete, correctness-gated observations.
pub trait EvidencePort {
    /// Write one complete document, refusing empty or resource-incomplete samples.
    ///
    /// # Errors
    /// Returns encoding, validation, stream-write or flush failures.
    fn publish(&self, publication: &MeasuredPublication<'_>) -> Result<(), Error>;
    /// Report the declared contract, both comparison dimensions and both producers' execution
    /// metadata beside the timing output, never as a benchmark metric.
    ///
    /// # Errors
    /// Returns stream-write or flush failures.
    fn comparison(
        &self,
        reference: &RunReport,
        candidate: &RunReport,
        comparison: ReportComparison,
        contract: ComparisonContract,
    ) -> Result<(), Error>;
    /// Add each project's candidate cost, `<language>/<project>`; nothing is written before `finish`.
    ///
    /// # Errors
    /// An empty set, or a project already added.
    fn costs(
        &self,
        language: &ryo_benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage,
        costs: &crate::domain::measurements::types::project_cost::value::ProjectCosts,
    ) -> Result<(), Error>;
    /// Add each rule's verification times, `<language>/verify/<rule>`, with the measures `total-ms`, `max-case-ms` and
    /// `compile-ms`; nothing is written before `finish`.
    ///
    /// # Errors
    /// A rule already added.
    fn verification(
        &self,
        language: &ryo_benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage,
        timings: &crate::outbound::evidence::types::RuleTimings,
    ) -> Result<(), Error>;
    /// Add each shape's growth, `<language>/growth/<shape>`, with the measures `doubling-ratio` and `larger-ms`; nothing is
    /// written before `finish`.
    ///
    /// # Errors
    /// A shape already added.
    fn growth(
        &self,
        language: &ryo_benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage,
        growth: &crate::domain::growth::types::ShapeGrowths,
    ) -> Result<(), Error>;
    /// Read the times the inventory's verification left in `files`, one per verified rule.
    ///
    /// # Errors
    /// A missing, unreadable or malformed file, or one that holds no times of its rule.
    fn read_verification(
        &self,
        files: &crate::outbound::evidence::types::VerificationTimesFiles,
    ) -> Result<crate::outbound::evidence::types::RuleTimings, Error>;
    /// Write the one document every measure added so far is in.
    ///
    /// # Errors
    /// An empty document, encoding, stream-write or flush failures.
    fn finish(&self) -> Result<(), Error>;
    /// Publish the differing-rule and unaccepted-difference counts of a corpus parity as metrics, and its
    /// per-rule document on the comparison stream. The counts of every parity published through this boundary
    /// are one document, each under its reference's name, written when the boundary is done.
    ///
    /// # Errors
    /// Returns encoding, stream-write or flush failures.
    fn parity(
        &self,
        parity: &ryo_benchmark_contracts::domain::reports::types::ParityReport<'_>,
    ) -> Result<(), Error>;
}
