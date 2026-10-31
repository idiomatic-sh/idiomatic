//! Compare full diagnostic multisets.
use crate::domain::diagnostics::types::Difference;
use crate::domain::diagnostics::types::RunReport;
/// Compare exact finding multisets and submitted-file coverage.
pub trait CompareUseCase {
    /// Require equal findings, including duplicates, over identical submitted inputs.
    ///
    /// # Errors
    ///
    /// Returns both sorted finding multisets when findings or submitted paths disagree.
    fn compare(&self, reference: &RunReport, candidate: &RunReport) -> Result<(), Difference>;
}
