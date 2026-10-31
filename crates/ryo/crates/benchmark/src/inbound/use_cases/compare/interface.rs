//! Compare full diagnostic multisets.
use crate::domain::diagnostics::types::Difference;
use crate::domain::diagnostics::types::RunReport;
pub trait CompareUseCase {
    fn compare(&self, reference: &RunReport, candidate: &RunReport) -> Result<(), Difference>;
}
