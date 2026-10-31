//! Exact normalized comparison has no external dependencies.
use super::interface::CompareUseCase;
use crate::domain::diagnostics::types::Difference;
use crate::domain::diagnostics::types::RunReport;
pub struct CompareUseCaseImpl;
impl CompareUseCaseImpl {
    pub fn new() -> Self {
        Self
    }
}
impl CompareUseCase for CompareUseCaseImpl {
    fn compare(&self, reference: &RunReport, candidate: &RunReport) -> Result<(), Difference> {
        super::service::compare(reference, candidate)
    }
}
