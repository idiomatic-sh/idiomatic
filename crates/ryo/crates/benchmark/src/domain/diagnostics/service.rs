//! Exact multisets preserve duplicated findings.
use crate::domain::diagnostics::types::Difference;
use crate::domain::diagnostics::types::RunReport;

/// Compare canonical finding multisets and exact submitted-file coverage.
///
/// # Errors
/// Returns both normalized finding multisets when their multiplicity or input coverage differs.
pub fn compare(reference: &RunReport, candidate: &RunReport) -> Result<(), Difference> {
    let mut left = reference.diagnostics.clone();
    let mut right = candidate.diagnostics.clone();
    left.sort();
    right.sort();
    let mut left_files = reference.files.clone();
    let mut right_files = candidate.files.clone();
    left_files.sort();
    right_files.sort();
    if left == right && left_files == right_files {
        Ok(())
    } else {
        Err(Difference {
            reference: left,
            candidate: right,
        })
    }
}
