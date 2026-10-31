//! Preparation and decoding are deliberately outside the measured interval.
use crate::domain::diagnostics::errors::DiagnosticsError as Error;
use crate::domain::measurements::types::Measurement;
use crate::domain::suite::types::RunRequest;
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::linter::interface::LinterPort;

/// Prepare and execute one contestant, then validate decoded input coverage.
pub fn run(
    linter: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    request: &RunRequest,
) -> Result<Measurement, Error> {
    let invocation = linter.prepare(request)?;
    let output = execution.run(invocation)?;
    let wall = output.wall;
    let resources = output.resources;
    let report = linter.decode(request, output)?;
    let mut expected = request.files.clone();
    expected.sort();
    if expected.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(Error::Invalid(
            crate::domain::diagnostics::constants::DUPLICATE_INPUT.into(),
        ));
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
    Ok(Measurement {
        wall,
        resources,
        report,
    })
}

use crate::domain::diagnostics::service::compare;
use crate::outbound::suite::interface::SuitePort;
use std::path::Path;
/// Verify each rule's independently authored findings through both selected contestants.
pub fn verify(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    rules: &Path,
    scratch: &Path,
) -> Result<crate::domain::suite::types::CaseCount, Error> {
    let cases = suite.cases(
        crate::outbound::suite::types::ArtifactPath(rules),
        crate::outbound::suite::types::ArtifactPath(scratch),
    )?;
    let count = cases.len();
    let first = cases
        .first()
        .ok_or_else(|| Error::Invalid(super::constants::NO_CASES.into()))?;
    // The normal candidate path loads and compiles the whole selected profile.
    // Reuse its first result; a startup failure must not repeat for every fixture.
    let mut initial = Some(run(candidate, execution, &first.request)?);
    let mut failures = Vec::new();
    for case in cases {
        for (name, tool) in [
            (super::constants::REFERENCE, reference),
            (super::constants::CANDIDATE, candidate),
        ] {
            let result = if name == super::constants::CANDIDATE {
                initial
                    .take()
                    .map_or_else(|| run(tool, execution, &case.request), Ok)
            } else {
                run(tool, execution, &case.request)
            };
            match result {
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
        Ok(crate::domain::suite::types::CaseCount(count))
    } else {
        Err(Error::Invalid(format!(
            "{count} cases checked; failures:\n{}",
            failures.join(super::constants::LINE_SEPARATOR)
        )))
    }
}

/// Verify a complete accounted profile before either contestant can execute a case.
pub fn verify_accounted(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    profile: &Path,
    scratch: &Path,
) -> Result<crate::domain::suite::types::CaseCount, Error> {
    let path = crate::outbound::suite::types::ArtifactPath(profile);
    let tool = suite.profile(path)?;
    let audit = suite.audit(path, &tool)?;
    crate::domain::suite::service::require_accounted(&audit, &tool)?;
    verify(
        suite,
        reference,
        candidate,
        execution,
        &profile.join(super::constants::RULES),
        scratch,
    )
}
