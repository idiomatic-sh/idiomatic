//! Measure after default coverage and per-sample correctness hold.
use crate::domain::measurements::types::Pair;
use crate::domain::suite::types::Audit;
use crate::domain::{diagnostics::errors::Error, suite::types::RunRequest};
pub trait MeasureUseCase {
    fn measure(&self, audit: &Audit, request: &RunRequest) -> Result<Pair, Error>;
}
