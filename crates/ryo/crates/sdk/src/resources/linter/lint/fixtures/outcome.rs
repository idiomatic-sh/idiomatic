//! The test data of `linter lint` that fixes outcome.

use crate::resources::linter::lint::types::LintReport;

/// What a scenario answers: how many findings, how many files nothing read, whether the repository holds nothing but what it held,
/// and how many columns the first finding carries.
pub(crate) type Outcome = (usize, usize, bool, usize);

/// What a scenario answers of its last report.
pub(crate) fn summary(report: &LintReport) -> Outcome {
    (
        report.findings.len(),
        report.unread.len(),
        true,
        report
            .findings
            .first()
            .map_or(0, |finding| finding.columns.len()),
    )
}
