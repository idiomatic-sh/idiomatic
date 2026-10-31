//! Measure after default coverage and per-sample correctness hold.
use crate::domain::measurements::types::Pair;
use crate::domain::suite::types::Audit;
use crate::domain::{diagnostics::errors::Error, suite::types::RunRequest};
/// Release paired process observations only after the correctness gate succeeds.
pub trait MeasureUseCase {
    /// Run both contestants and compare their complete canonical reports.
    ///
    /// # Errors
    ///
    /// Returns an error for incomplete coverage, operational failure or unequal results.
    fn measure(&self, audit: &Audit, request: &RunRequest) -> Result<Pair, Error>;
}
