//! A corpus is measured only after both contestants are certified on it.
use super::types::CorpusMeasurement;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
/// Measure the corpus of a certified pair and publish or report its samples.
pub trait MeasureCorpusUseCase {
    /// Measure every project, then publish the samples of an accounted run, or say that a
    /// diagnostic run is not a metric; every observation is written to `observations`.
    ///
    /// # Errors
    /// Refuses the first unsound observation of a project after measuring the others, and a
    /// failed publication.
    fn measure_corpus(
        &self,
        corpus: &CorpusMeasurement<'_>,
        observations: &mut dyn std::io::Write,
    ) -> Result<(), ReportsError>;
}
