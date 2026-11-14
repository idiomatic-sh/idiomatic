//! The sources the pinned reference refuses to parse, read from one reference run over the whole request and dropped from it.
use ryo_benchmark_contracts::domain::reports::types::{BenchmarkFinding, RunReport};

/// Drops from `files` each source the reference's `report` names under its invalid-syntax rule `syntax`, and answers those
/// findings, in the report's order, for the caller to report.
pub(crate) fn exclude_refused(
    report: &RunReport,
    syntax: &ryo::linter::RuleId,
    files: &mut Vec<ryo::linter::RelativePath>,
) -> Vec<BenchmarkFinding> {
    let refused: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|finding| finding.rule == *syntax)
        .cloned()
        .collect();
    let invalid: std::collections::BTreeSet<_> =
        refused.iter().map(|finding| &finding.file).collect();
    files.retain(|file| !invalid.contains(file));
    refused
}
