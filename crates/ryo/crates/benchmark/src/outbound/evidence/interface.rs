//! Asymptote owns the ratchet; this port publishes only observed samples.
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
    fn comparison(
        &self,
        reference: &RunReport,
        candidate: &RunReport,
        comparison: ReportComparison,
        contract: ComparisonContract,
    );
}
