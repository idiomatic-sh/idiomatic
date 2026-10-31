//! Case orchestration depends only on suite, execution and linter contracts.
use crate::application::run::service::run;
use crate::domain::diagnostics::errors::Error;
use crate::domain::diagnostics::service::compare;
use crate::outbound::{
    execution::interface::ExecutionPort, linter::interface::LinterPort, suite::interface::SuitePort,
};
use std::path::Path;

/// Verify each rule's independently authored findings through both selected contestants.
pub fn verify(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    rules: &Path,
    scratch: &Path,
) -> Result<usize, Error> {
    let cases = suite.cases(rules, scratch)?;
    let count = cases.len();
    let mut failures = Vec::new();
    for case in cases {
        for (name, tool) in [("reference", reference), ("candidate", candidate)] {
            match run(tool, execution, &case.request) {
                Ok(mut result) => {
                    result
                        .report
                        .diagnostics
                        .retain(|finding| finding.rule == case.rule);
                    if let Err(difference) = compare(&case.expected, &result.report) {
                        failures.push(format!(
                            "{}/{} expected-vs-{name}: expected={:?}; actual={:?}",
                            case.rule, case.name, difference.reference, difference.candidate
                        ));
                    }
                }
                Err(error) => failures.push(format!("{}/{} {name}: {error}", case.rule, case.name)),
            }
        }
    }
    if failures.is_empty() {
        Ok(count)
    } else {
        Err(Error::Invalid(format!(
            "{count} cases checked; failures:\n{}",
            failures.join("\n")
        )))
    }
}
