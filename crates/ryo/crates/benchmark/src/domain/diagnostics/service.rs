//! Independent diagnostic and execution evidence; unknown metadata is never a clean execution.
use ryo_benchmark_contracts::domain::reports::types::Comparison;
use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
use ryo_benchmark_contracts::domain::reports::types::Difference;
use ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata;
use ryo_benchmark_contracts::domain::reports::types::ReportComparison;
use ryo_benchmark_contracts::domain::reports::types::RunReport;

/// Validate explicitly complete candidate execution without inferring skipped work from diagnostics.
///
/// # Errors
/// Refuses missing capability/coverage, unknown work, inconsistent counts and invalid or duplicated skips.
pub fn require_complete_execution(report: &RunReport) -> Result<(), &'static str> {
    if report.execution_metadata != ExecutionMetadata::Complete {
        return Err("candidate execution metadata is not explicitly complete");
    }
    let coverage = report
        .coverage
        .ok_or("complete execution metadata lacks coverage")?;
    let files = report
        .files
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    if files.is_empty()
        || files.len() != report.files.len()
        || coverage.selected < files.len()
        || coverage.unknown_or_unread != 0
        || coverage.evaluated.checked_add(coverage.skipped) != Some(coverage.selected)
        || coverage.skipped != report.skipped.len()
    {
        return Err("candidate execution coverage is incomplete or inconsistent");
    }
    let mut pairs = std::collections::BTreeSet::new();
    for skip in &report.skipped {
        if !report.files.contains(&skip.path)
            || !pairs.insert((skip.rule.as_ref(), skip.path.0.as_ref()))
        {
            return Err("candidate execution contains invalid or duplicate skipped pairs");
        }
    }
    Ok(())
}

/// Compare only authored diagnostic expectations and exact submitted files, not execution claims.
///
/// # Errors
/// Returns both raw finding multisets and skip arrays without discarding evidence.
pub fn compare_diagnostics(reference: &RunReport, candidate: &RunReport) -> Result<(), Difference> {
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
            reference_skipped: reference.skipped.clone(),
            candidate_skipped: candidate.skipped.clone(),
        })
    }
}

/// Report independently comparable dimensions; candidate invariants apply even when the reference cannot report execution.
#[must_use]
pub fn compare_dimensions(reference: &RunReport, candidate: &RunReport) -> ReportComparison {
    let diagnostics = if compare_diagnostics(reference, candidate).is_ok() {
        Comparison::Exact
    } else {
        Comparison::Different
    };
    let execution_metadata = if require_complete_execution(candidate).is_err() {
        Comparison::Different
    } else if reference.execution_metadata != ExecutionMetadata::Complete {
        Comparison::Incomparable
    } else if require_complete_execution(reference).is_err() {
        Comparison::Different
    } else {
        let pairs = |report: &RunReport| {
            report
                .skipped
                .iter()
                .map(|skip| {
                    (
                        skip.rule.as_ref().to_owned(),
                        skip.path.0.as_ref().to_owned(),
                        skip.reason,
                    )
                })
                .collect::<std::collections::BTreeSet<_>>()
        };
        if reference.coverage == candidate.coverage && pairs(reference) == pairs(candidate) {
            Comparison::Exact
        } else {
            Comparison::Different
        }
    };
    ReportComparison {
        diagnostics,
        execution_metadata,
    }
}

/// Apply an explicit comparison contract. Unavailable reference metadata may support diagnostic timing, never full parity.
///
/// # Errors
/// Refuses diagnostic differences, invalid candidate execution, known metadata differences, or unproven required full parity.
pub fn compare_with_contract(
    reference: &RunReport,
    candidate: &RunReport,
    contract: ComparisonContract,
) -> Result<ReportComparison, Difference> {
    let dimensions = compare_dimensions(reference, candidate);
    if dimensions.diagnostics == Comparison::Exact
        && dimensions.execution_metadata != Comparison::Different
        && (contract == ComparisonContract::Diagnostics
            || dimensions.execution_metadata == Comparison::Exact)
    {
        Ok(dimensions)
    } else {
        Err(compare_diagnostics(reference, candidate)
            .err()
            .unwrap_or_else(|| Difference {
                reference: reference.diagnostics.clone(),
                candidate: candidate.diagnostics.clone(),
                reference_skipped: reference.skipped.clone(),
                candidate_skipped: candidate.skipped.clone(),
            }))
    }
}

/// Require exact diagnostics and explicitly comparable execution metadata.
///
/// # Errors
/// Refuses missing execution evidence as well as actual differences.
pub fn compare(reference: &RunReport, candidate: &RunReport) -> Result<(), Difference> {
    compare_with_contract(reference, candidate, ComparisonContract::CompleteReport).map(|_| ())
}
