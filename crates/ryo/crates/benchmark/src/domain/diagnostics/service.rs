//! Independent diagnostic and execution evidence; unknown metadata is never a clean execution.
use ryo_benchmark_contracts::domain::reports::service::require_complete_execution;
use ryo_benchmark_contracts::domain::reports::types::Comparison;
use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
use ryo_benchmark_contracts::domain::reports::types::Difference;
use ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata;
use ryo_benchmark_contracts::domain::reports::types::ReportComparison;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::reports::types::{BenchmarkFinding, SourceByteOffset};

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

/// Require one tool's reports of the same sources under two cache conditions to agree: exact diagnostics and files, and
/// complete, equal execution metadata unless the tool's format reports none in either run.
///
/// # Errors
/// Refuses what `compare` refuses; for two reports whose execution metadata is unavailable, only a diagnostic or file difference.
pub fn compare_conditions(first: &RunReport, second: &RunReport) -> Result<(), Difference> {
    let unavailable = |report: &RunReport| {
        matches!(
            report.execution_metadata,
            ExecutionMetadata::Unavailable { .. }
        )
    };
    if unavailable(first) && unavailable(second) {
        compare_diagnostics(first, second)
    } else {
        compare(first, second)
    }
}

/// Validate decoded coverage without treating a rejected report as a process failure.
///
/// # Errors
/// Refuses duplicate requested inputs, a report whose files differ from the request, and a diagnostic outside the files or with an inverted span.
pub fn validate_coverage(
    requested: &[ryo::linter::RelativePath],
    report: &RunReport,
) -> Result<(), ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
    use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
    let mut expected = requested.to_vec();
    expected.sort();
    if expected
        .windows(2)
        .any(|pair| matches!(pair, [first, second] if first == second))
    {
        return Err(Error::Invalid(super::constants::DUPLICATE_INPUT.into()));
    }
    let mut actual = report.files.clone();
    actual.sort();
    if actual != expected {
        return Err(Error::Invalid(super::constants::INPUT_COVERAGE.into()));
    }
    for finding in &report.diagnostics {
        if actual.binary_search(&finding.file).is_err() || finding.start > finding.end {
            return Err(Error::Invalid(format!("invalid diagnostic: {finding:?}")));
        }
    }
    Ok(())
}

/// Compare the findings of each rule as a multiset on file and start: the selected rules first, in
/// selection order, then every other rule either report holds, in rule order, so no finding is left out.
///
/// At one file and start, findings pair in report order; the reference's unpaired findings are missing
/// and the candidate's are extra.
#[must_use]
pub(crate) fn compare_rules(
    selected: &[ryo::linter::RuleId],
    reference: &RunReport,
    candidate: &RunReport,
) -> Vec<ryo_benchmark_contracts::domain::reports::types::RuleComparison<BenchmarkFinding>> {
    use ryo_benchmark_contracts::domain::reports::types::{EndPair, FindingCount, RuleComparison};
    let theirs = sites(reference);
    let ours = sites(candidate);
    let empty = Sites::new();
    let chosen: std::collections::BTreeSet<_> = selected.iter().collect();
    let outside: std::collections::BTreeSet<_> = theirs
        .keys()
        .chain(ours.keys())
        .copied()
        .filter(|rule| !chosen.contains(rule))
        .collect();
    selected
        .iter()
        .chain(outside)
        .map(|rule| {
            let reference_sites = theirs.get(rule).unwrap_or(&empty);
            let candidate_sites = ours.get(rule).unwrap_or(&empty);
            let mut comparison = RuleComparison {
                rule: rule.clone(),
                matched: FindingCount(0),
                missing: Vec::new(),
                extra: Vec::new(),
                end_differences: Vec::new(),
            };
            let keys: std::collections::BTreeSet<_> = reference_sites
                .keys()
                .chain(candidate_sites.keys())
                .collect();
            for key in keys {
                let left = candidate_sites.get(key).map_or(&[][..], Vec::as_slice);
                let right = reference_sites.get(key).map_or(&[][..], Vec::as_slice);
                let matched = left.len().min(right.len());
                comparison.matched.0 += matched;
                for (ours, theirs) in left.iter().zip(right) {
                    if ours.end != theirs.end {
                        comparison.end_differences.push(EndPair {
                            candidate: (*ours).clone(),
                            reference: (*theirs).clone(),
                        });
                    }
                }
                comparison
                    .missing
                    .extend(right.iter().skip(matched).map(|finding| (*finding).clone()));
                comparison
                    .extra
                    .extend(left.iter().skip(matched).map(|finding| (*finding).clone()));
            }
            comparison
        })
        .collect()
}

/// One rule's findings by file and start, in report order.
type Sites<'a> = std::collections::BTreeMap<
    (&'a ryo::linter::RelativePath, SourceByteOffset),
    Vec<&'a BenchmarkFinding>,
>;

/// A report's findings by rule, then by file and start.
fn sites(report: &RunReport) -> std::collections::BTreeMap<&ryo::linter::RuleId, Sites<'_>> {
    let mut sites = std::collections::BTreeMap::<_, Sites<'_>>::new();
    for finding in &report.diagnostics {
        sites
            .entry(&finding.rule)
            .or_default()
            .entry((&finding.file, finding.start))
            .or_default()
            .push(finding);
    }
    sites
}

/// The one-based line and Unicode-character column of a UTF-8 byte offset, lines split on `\r\n`, `\r` or `\n`.
///
/// # Errors
/// Refuses an offset past the end of the source or inside a character.
pub(crate) fn position(
    source: &ryo::linter::SourceText,
    offset: SourceByteOffset,
) -> Result<
    ryo_benchmark_contracts::domain::reports::types::FindingPosition,
    ryo_benchmark_contracts::domain::reports::errors::ReportsError,
> {
    use ryo_benchmark_contracts::domain::reports::types::{
        FindingPosition, SourceColumn, SourceLine,
    };
    let outside = || {
        ryo_benchmark_contracts::domain::reports::errors::ReportsError::Invalid(
            super::constants::OFFSET_OUTSIDE_SOURCE.into(),
        )
    };
    let text = source.0.as_ref();
    let prefix = text.get(..offset.0).ok_or_else(outside)?;
    let mut line = 1_usize;
    let mut start = 0_usize;
    let mut index = 0_usize;
    while let Some(byte) = prefix.as_bytes().get(index) {
        index += 1;
        match byte {
            b'\r' if text.as_bytes().get(index) == Some(&b'\n') => {
                index += 1;
                if index <= prefix.len() {
                    line += 1;
                    start = index;
                }
            }
            b'\r' | b'\n' => {
                line += 1;
                start = index;
            }
            _ => {}
        }
    }
    let column = prefix.get(start..).ok_or_else(outside)?.chars().count() + 1;
    Ok(FindingPosition {
        line: SourceLine(line),
        column: SourceColumn(column),
    })
}
