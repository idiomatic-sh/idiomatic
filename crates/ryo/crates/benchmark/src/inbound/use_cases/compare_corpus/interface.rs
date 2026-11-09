//! What the `compare_corpus` use case promises its callers, in the core's own vocabulary.
use super::types::CorpusParity;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;

/// What `compare_corpus` offers its callers.
pub trait CompareCorpusUseCase {
    /// Run both contestants once over the request, compare each configured rule's findings, judge every
    /// difference against the accepted differences of the corpus, and publish the per-rule parity;
    /// the summary and every problem are written to `observations`.
    ///
    /// # Errors
    /// Refuses a request that does not select exactly the configured rules and accepted differences
    /// written for another corpus, both before either contestant runs; fails a contestant run, and
    /// fails after publishing when any difference is not accepted.
    fn compare_corpus(
        &self,
        parity: &CorpusParity<'_>,
        observations: &mut dyn std::io::Write,
    ) -> Result<(), ReportsError>;
}
