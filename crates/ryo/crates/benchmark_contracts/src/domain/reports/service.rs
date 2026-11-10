//! Execution completeness is an invariant of the shared report, independent of transport.
use super::errors::ReportsError;
use super::types::{CoverageAccounting, ExecutionMetadata, RunReport};

/// Validate explicitly complete candidate execution without inferring skipped work from diagnostics.
///
/// # Errors
/// Refuses missing capability/coverage, unknown work, inconsistent counts and invalid or duplicated skips.
pub fn require_complete_execution(report: &RunReport) -> Result<(), ReportsError> {
    if report.execution_metadata != ExecutionMetadata::Complete {
        return Err(ReportsError::Invalid(
            super::constants::INCOMPLETE_METADATA.into(),
        ));
    }
    let CoverageAccounting::Accounted(coverage) = report.coverage else {
        return Err(ReportsError::Invalid(
            super::constants::MISSING_COVERAGE.into(),
        ));
    };
    let files = report
        .files
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    if files.is_empty()
        || files.len() != report.files.len()
        || coverage.selected.0 < files.len()
        || coverage.unknown_or_unread.0 != 0
        || coverage.evaluated.0.checked_add(coverage.skipped.0) != Some(coverage.selected.0)
        || coverage.skipped.0 != report.skipped.len()
    {
        return Err(ReportsError::Invalid(
            super::constants::INCONSISTENT_COVERAGE.into(),
        ));
    }
    let mut pairs = std::collections::BTreeSet::new();
    for skip in &report.skipped {
        if !report.files.contains(&skip.path)
            || !pairs.insert((skip.rule.as_ref(), skip.path.0.as_ref()))
        {
            return Err(ReportsError::Invalid(
                super::constants::INVALID_SKIPS.into(),
            ));
        }
    }
    Ok(())
}
