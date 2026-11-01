//! Preparation and decoding are deliberately outside the measured interval.
use crate::domain::diagnostics::errors::DiagnosticsError as Error;
use crate::domain::measurements::types::Measurement;
use crate::domain::suite::types::RunRequest;
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::linter::interface::LinterPort;

/// Prepare and execute one contestant, then validate decoded input coverage.
///
/// # Errors
/// Propagates preparation, launch or decoding errors and refuses invalid input coverage or ranges.
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
    if expected
        .windows(2)
        .any(|pair| matches!(pair, [first, second] if first == second))
    {
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
///
/// # Errors
/// Refuses missing cases, startup failures and differences against independently authored findings.
pub fn verify(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    rules: &Path,
    scratch: &Path,
) -> Result<crate::domain::suite::types::CaseCount, Error> {
    verify_shard(suite, reference, candidate, execution, rules, scratch, None)
}

/// A deterministic partition of the globally audited case inventory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaseShard {
    /// Zero-based shard number.
    pub index: usize,
    /// Number of disjoint shards.
    pub count: usize,
}

impl CaseShard {
    /// Validate shard coordinates.
    pub fn new(index: usize, count: usize) -> Result<Self, Error> {
        if count == 0 || index >= count {
            return Err(Error::Invalid("invalid case shard coordinates".into()));
        }
        Ok(Self { index, count })
    }
}

/// Verify one deterministic shard after loading and validating the entire case inventory.
///
/// Cases are ordered by `(rule, name)` before assigning index modulo shard count. The
/// suite loader and profile accounting run over the full inventory before this partition.
pub fn verify_shard(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    rules: &Path,
    scratch: &Path,
    shard: Option<CaseShard>,
) -> Result<crate::domain::suite::types::CaseCount, Error> {
    let cases = suite.cases(
        crate::outbound::suite::types::ArtifactPath(rules),
        crate::outbound::suite::types::ArtifactPath(scratch),
    )?;
    let mut cases = cases;
    cases.sort_by(|left, right| (&left.rule, &left.name).cmp(&(&right.rule, &right.name)));
    let global_count = cases.len();
    let cases = match shard {
        Some(shard) => cases
            .into_iter()
            .enumerate()
            .filter_map(|(index, case)| (index % shard.count == shard.index).then_some(case))
            .collect::<Vec<_>>(),
        None => cases,
    };
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
            write_case_observation(&case, name, &result)?;
            match result {
                Ok(mut result) => {
                    if case.comparison == crate::domain::suite::types::CaseComparison::Rule {
                        result.report.diagnostics.retain(|finding| {
                            finding.rule == case.rule
                                || finding.rule.as_ref()
                                    == crate::domain::diagnostics::constants::FRONTEND_SYNTAX
                        });
                    }
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
        debug_assert!(count <= global_count);
        Ok(crate::domain::suite::types::CaseCount(count))
    } else {
        Err(Error::Invalid(format!(
            "{count} cases checked; failures:\n{}",
            failures.join(super::constants::LINE_SEPARATOR)
        )))
    }
}

fn write_case_observation(
    case: &crate::domain::suite::types::PreparedCase,
    side: &str,
    result: &Result<Measurement, Error>,
) -> Result<(), Error> {
    let Some(path) = std::env::var_os("RYO_VERIFY_CASE_RESULTS") else {
        return Ok(());
    };
    let findings = result.as_ref().ok().map(|measurement| {
        measurement
            .report
            .diagnostics
            .iter()
            .map(|finding| {
                serde_json::json!({
                    "rule": finding.rule.as_ref(),
                    "file": finding.file.0.as_ref(),
                    "start": finding.start.0,
                    "end": finding.end.0,
                })
            })
            .collect::<Vec<_>>()
    });
    let files = result.as_ref().ok().map(|measurement| {
        measurement
            .report
            .files
            .iter()
            .map(|file| file.0.as_ref())
            .collect::<Vec<_>>()
    });
    let error = result.as_ref().err().map(ToString::to_string);
    let expected = case
        .expected
        .diagnostics
        .iter()
        .map(|finding| {
            serde_json::json!({
                "rule": finding.rule.as_ref(),
                "file": finding.file.0.as_ref(),
                "start": finding.start.0,
                "end": finding.end.0,
            })
        })
        .collect::<Vec<_>>();
    let mut source_hasher = sha2::Sha256::new();
    for file in &case.request.files {
        use sha2::Digest as _;
        source_hasher.update(file.0.as_bytes());
        source_hasher.update([0]);
        source_hasher.update(std::fs::read(case.request.root.join(file.0.as_ref()))?);
    }
    use sha2::Digest as _;
    let record = serde_json::json!({
        "rule": case.rule.as_ref(),
        "name": case.name.to_string(),
        "side": side,
        "files": files,
        "diagnostics": findings,
        "expected": expected,
        "source_sha256": format!("{:x}", source_hasher.finalize()),
        "error": error,
        "complete": true,
    });
    let mut line =
        serde_json::to_vec(&record).map_err(|error| Error::Invalid(error.to_string()))?;
    line.push(b'\n');
    use std::io::Write as _;
    let mut output = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    output.write_all(&line)?;
    output.sync_data()?;
    Ok(())
}

/// Check independently authored cases against only the reference executable.
///
/// This is useful while preparing or auditing expected fixtures, when a candidate
/// query may not yet compile. It does not compare a candidate and cannot produce a
/// correctness certificate.
///
/// # Errors
/// Refuses missing cases, reference launch failures, or differences from authored findings.
pub fn verify_reference_cases(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    rules: &Path,
    scratch: &Path,
) -> Result<crate::domain::suite::types::ReferenceCaseCount, Error> {
    let cases = suite.cases(
        crate::outbound::suite::types::ArtifactPath(rules),
        crate::outbound::suite::types::ArtifactPath(scratch),
    )?;
    let count = cases.len();
    if cases.is_empty() {
        return Err(Error::Invalid(super::constants::NO_CASES.into()));
    }
    let mut failures = Vec::new();
    for case in cases {
        match run(reference, execution, &case.request) {
            Ok(mut result) => {
                if case.comparison == crate::domain::suite::types::CaseComparison::Rule {
                    result.report.diagnostics.retain(|finding| {
                        finding.rule == case.rule
                            || finding.rule.as_ref()
                                == crate::domain::diagnostics::constants::FRONTEND_SYNTAX
                    });
                }
                if let Err(difference) = compare(&case.expected, &result.report) {
                    failures.push(format!(
                        "{}/{} expected-vs-reference: expected={:?}; actual={:?}",
                        case.rule, case.name, difference.reference, difference.candidate
                    ));
                }
            }
            Err(error) => failures.push(format!("{}/{} reference: {error}", case.rule, case.name)),
        }
    }
    if failures.is_empty() {
        Ok(crate::domain::suite::types::ReferenceCaseCount(count))
    } else {
        Err(Error::Invalid(format!(
            "{count} reference-only cases checked; failures:\n{}",
            failures.join(super::constants::LINE_SEPARATOR)
        )))
    }
}

/// Verify a complete accounted profile before either contestant can execute a case.
///
/// # Errors
/// Refuses unaccounted defaults before executing any case and propagates verification failures.
pub fn verify_accounted(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    profile: &Path,
    scratch: &Path,
) -> Result<crate::domain::suite::types::CaseCount, Error> {
    verify_accounted_shard(
        suite, reference, candidate, execution, profile, scratch, None,
    )
}

/// Verify the full profile accounting and then execute one deterministic case shard.
pub fn verify_accounted_shard(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    profile: &Path,
    scratch: &Path,
    shard: Option<CaseShard>,
) -> Result<crate::domain::suite::types::CaseCount, Error> {
    let path = crate::outbound::suite::types::ArtifactPath(profile);
    let tool = suite.profile(path)?;
    let audit = suite.audit(path, &tool)?;
    crate::domain::suite::service::require_accounted(&audit, &tool)?;
    verify_shard(
        suite,
        reference,
        candidate,
        execution,
        &profile.join(super::constants::RULES),
        scratch,
        shard,
    )
}

#[cfg(test)]
mod shard_tests {
    use super::CaseShard;

    #[test]
    fn shard_coordinates_are_zero_based_and_bounded() {
        assert!(matches!(
            CaseShard::new(0, 3),
            Ok(CaseShard { index: 0, count: 3 })
        ));
        assert!(matches!(
            CaseShard::new(2, 3),
            Ok(CaseShard { index: 2, count: 3 })
        ));
        assert!(CaseShard::new(3, 3).is_err());
        assert!(CaseShard::new(0, 0).is_err());
    }
}
