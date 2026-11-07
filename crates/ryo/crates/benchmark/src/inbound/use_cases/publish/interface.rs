//! The publication operation accepts only the owning workflow's immutable samples.
use crate::domain::measurements::types::sample::value::BenchmarkSample;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
/// Publish checked complete-inventory observations without exposing proof construction.
pub trait PublishUseCase {
    /// # Errors
    /// Refuses diagnostic/empty samples before the destination can write anything.
    fn publish(&self, samples: &[BenchmarkSample]) -> Result<(), ReportsError>;
}
