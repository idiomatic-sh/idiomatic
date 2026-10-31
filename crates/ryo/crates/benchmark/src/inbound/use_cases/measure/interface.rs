//! Measure after default coverage and per-sample correctness hold.
use crate::domain::measurements::types::{RunOrder, pair::value::Pair};
use crate::domain::suite::types::certificate::value::Certificate;
use crate::domain::{diagnostics::errors::DiagnosticsError as Error, suite::types::RunRequest};
/// Release paired process observations only after the correctness gate succeeds.
pub trait MeasureUseCase {
    /// Run both contestants and compare their complete canonical reports.
    ///
    /// # Errors
    ///
    /// Returns an error for incomplete coverage, operational failure or unequal results.
    fn measure(
        &self,
        certificate: &Certificate,
        request: &RunRequest,
        order: RunOrder,
    ) -> Result<Pair, Error>;
}
