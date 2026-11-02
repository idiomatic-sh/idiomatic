//! Compare canonical reports through the diagnostics domain.
use crate::domain::diagnostics::types::{Difference, RunReport};
/// Require exact diagnostic multisets, input files and explicitly complete execution metadata.
pub(super) fn compare(reference: &RunReport, candidate: &RunReport) -> Result<(), Difference> {
    crate::domain::diagnostics::service::compare(reference, candidate)
}
