//! Asymptote owns the ratchet; this port gathers one document of observed measures and writes it once.
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::reports::types::{
    ComparisonContract, ReportComparison, RunReport,
};

/// The one writer of a bench's document, and the comparison evidence beside it.
pub trait EvidencePort {
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
        language: &benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage,
        costs: &benchmark_contracts::domain::costs::types::ProjectCosts,
    ) -> Result<(), Error>;
    /// Add each rule's verification times, `<language>/verify/<rule>`, with the measures `total-ms`, `max-case-ms` and
    /// `compile-ms`; nothing is written before `finish`.
    ///
    /// # Errors
    /// A rule already added.
    fn verification(
        &self,
        language: &benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage,
        timings: &crate::outbound::evidence::types::RuleTimings,
    ) -> Result<(), Error>;
    /// Add each shape's growth, `<language>/growth/<shape>`, with the measures `doubling-ratio` and `larger-ms`; nothing is
    /// written before `finish`.
    ///
    /// # Errors
    /// A shape already added.
    fn growth(
        &self,
        language: &benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage,
        growth: &benchmark_contracts::domain::measurements::types::ShapeGrowths,
    ) -> Result<(), Error>;
    /// Read the times the inventory's verification left in `files`, one per verified rule.
    ///
    /// # Errors
    /// A missing, unreadable or malformed file, or one that holds no times of its rule.
    fn read_verification(
        &self,
        files: &crate::outbound::evidence::types::VerificationTimesFiles,
    ) -> Result<crate::outbound::evidence::types::RuleTimings, Error>;
    /// The written `document`, one BMF line, with the measures of `class` alone and a benchmark left with none of them
    /// dropped.
    ///
    /// # Errors
    /// A document that does not read, a measure of no class, or a document that keeps no measure of `class`.
    fn select(
        &self,
        document: &crate::outbound::evidence::types::PublishedDocument,
        class: crate::outbound::evidence::types::MeasureClass,
    ) -> Result<crate::outbound::evidence::types::PublishedDocument, Error>;
    /// Write the one document every measure added so far is in.
    ///
    /// # Errors
    /// An empty document, encoding, stream-write or flush failures.
    fn finish(&self) -> Result<(), Error>;
}
