//! Preparation and decoding are deliberately outside the measured interval.
use crate::domain::diagnostics::{errors::DiagnosticsError as Error, types::RunReport};
use crate::domain::measurements::types::Measurement;
use crate::domain::suite::types::RunRequest;
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::linter::interface::LinterPort;
use sha2::Digest as _;
use std::io::Write as _;

/// Prepare and execute one contestant, then validate decoded input coverage.
///
/// # Errors
/// Propagates preparation, launch or decoding errors and refuses invalid input coverage or ranges.
pub fn run(
    linter: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    request: &RunRequest,
) -> Result<Measurement, Error> {
    if let Some(context) = &request.context_manifest {
        crate::outbound::context::adapters::local::adapter::validate(context, &request.root)?;
    }
    let invocation = linter.prepare(request)?;
    let output = execution.run(invocation)?;
    let wall = output.wall;
    let resources = output.resources;
    let report = linter.decode(request, output)?;
    if let Some(context) = &request.context_manifest {
        crate::outbound::context::adapters::local::adapter::validate(context, &request.root)?;
    }
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
    verify_shard(
        suite,
        reference,
        candidate,
        execution,
        rules,
        scratch,
        CaseSelection::default(),
    )
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
    ///
    /// # Errors
    /// Refuses a zero shard count or an index outside that count.
    pub fn new(index: usize, count: usize) -> Result<Self, Error> {
        if count == 0 || index >= count {
            return Err(Error::Invalid("invalid case shard coordinates".into()));
        }
        Ok(Self { index, count })
    }
}

/// Optional partition and durable case IDs for a verification invocation.
#[derive(Clone, Copy, Debug, Default)]
pub struct CaseSelection<'a> {
    /// Globally audited deterministic shard, or all cases.
    pub shard: Option<CaseShard>,
    /// An explicit subset of IDs assigned to this shard, or the whole shard.
    pub ids: Option<&'a std::collections::BTreeSet<String>>,
}

/// Audited global inventory and its exact selected partition.
struct Selection {
    /// Number of cases before partitioning.
    global_count: usize,
    /// Cases admitted to this invocation, preserving deterministic order.
    selected_cases: Vec<crate::domain::suite::types::PreparedCase>,
}

/// Select an exact, nonempty subset only after the complete inventory is loaded.
fn select_cases(
    mut cases: Vec<crate::domain::suite::types::PreparedCase>,
    selection: CaseSelection<'_>,
) -> Result<Selection, Error> {
    cases.sort_by(|left, right| (&left.rule, &left.name).cmp(&(&right.rule, &right.name)));
    let global_count = cases.len();
    let assigned = match selection.shard {
        Some(shard) => cases
            .into_iter()
            .enumerate()
            .filter_map(|(index, case)| (index % shard.count == shard.index).then_some(case))
            .collect::<Vec<_>>(),
        None => cases,
    };
    let assigned_ids = assigned
        .iter()
        .map(|case| format!("{}/{}", case.rule, case.name))
        .collect::<std::collections::BTreeSet<_>>();
    let selected_cases = match selection.ids {
        Some(selected) if selected.is_empty() || !selected.is_subset(&assigned_ids) => {
            return Err(Error::Invalid(
                "case selection is empty or outside this shard".into(),
            ));
        }
        Some(selected) => assigned
            .into_iter()
            .filter(|case| selected.contains(&format!("{}/{}", case.rule, case.name)))
            .collect::<Vec<_>>(),
        None => assigned,
    };
    Ok(Selection {
        global_count,
        selected_cases,
    })
}

/// Verify one deterministic shard after loading and validating the entire case inventory.
///
/// Cases are ordered by `(rule, name)` before assigning index modulo shard count. The
/// suite loader and profile accounting run over the full inventory before this partition.
///
/// # Errors
/// Propagates inventory, execution and observation errors, and refuses mismatched case reports.
pub fn verify_shard(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    rules: &Path,
    scratch: &Path,
    selection: CaseSelection<'_>,
) -> Result<crate::domain::suite::types::CaseCount, Error> {
    let mut cases = suite.cases(
        crate::outbound::suite::types::ArtifactPath(rules),
        crate::outbound::suite::types::ArtifactPath(scratch),
    )?;
    super::discrepancy::attach(suite, rules, &mut cases)?;
    let Selection {
        global_count,
        selected_cases,
    } = select_cases(cases, selection)?;
    let count = selected_cases.len();
    let first = selected_cases
        .first()
        .ok_or_else(|| Error::Invalid(super::constants::NO_CASES.into()))?;
    let validations = super::discrepancy::validate_all(suite, reference, rules, &selected_cases)?;
    // The normal candidate path loads and compiles the whole selected profile.
    // Reuse its first result; a startup failure must not repeat for every fixture.
    let mut initial = Some(run(candidate, execution, &first.request)?);
    let mut failures = Vec::new();
    for (case, validation) in selected_cases.into_iter().zip(validations) {
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
            let full_report = result
                .as_ref()
                .ok()
                .map(|measurement| measurement.report.clone());
            match result {
                Ok(mut result) => {
                    if case.comparison == crate::domain::suite::types::CaseComparison::Rule {
                        result.report.diagnostics.retain(|finding| {
                            finding.rule == case.rule
                                || finding.rule.as_ref()
                                    == crate::domain::diagnostics::constants::FRONTEND_SYNTAX
                        });
                    }
                    let expected = if name == super::constants::REFERENCE {
                        super::discrepancy::reference_expected(&case)
                    } else {
                        &case.expected
                    };
                    super::discrepancy::complete(
                        &case,
                        &result.report,
                        name == super::constants::CANDIDATE,
                    )?;
                    let comparison = compare(expected, &result.report);
                    write_case_observation(
                        &case,
                        name,
                        full_report.as_ref(),
                        None,
                        comparison.is_ok(),
                        validation.as_ref(),
                    )?;
                    if let Err(difference) = comparison {
                        failures.push(format!(
                            "{}/{} expected-vs-{name}: expected={:?}; actual={:?}; expected-skipped={:?}; actual-skipped={:?}",
                            case.rule,
                            case.name,
                            difference.reference,
                            difference.candidate,
                            difference.reference_skipped,
                            difference.candidate_skipped
                        ));
                    }
                }
                Err(error) => {
                    write_case_observation(
                        &case,
                        name,
                        None,
                        Some(&error.to_string()),
                        false,
                        validation.as_ref(),
                    )?;
                    failures.push(format!("{}/{} {name}: {error}", case.rule, case.name));
                }
            }
        }
    }
    if failures.is_empty() {
        debug_assert!(
            count <= global_count,
            "selected cases must belong to the audited inventory"
        );
        Ok(crate::domain::suite::types::CaseCount(count))
    } else {
        Err(Error::Invalid(format!(
            "{count} cases checked; failures:\n{}",
            failures.join(super::constants::LINE_SEPARATOR)
        )))
    }
}

/// Append one complete side record while preserving the unprojected report and source identity.
fn write_case_observation(
    case: &crate::domain::suite::types::PreparedCase,
    side: &str,
    report: Option<&RunReport>,
    error: Option<&str>,
    passed: bool,
    validation: Option<&serde_json::Value>,
) -> Result<(), Error> {
    let Some(path) = std::env::var_os("RYO_VERIFY_CASE_RESULTS") else {
        return Ok(());
    };
    let findings = report.map(|report| finding_values(&report.diagnostics));
    let files = report.map(|report| {
        report
            .files
            .iter()
            .map(|file| file.0.as_ref())
            .collect::<Vec<_>>()
    });
    let skipped = report.map(|report| {
        report
            .skipped
            .iter()
            .map(|skip| {
                serde_json::json!({
                    "rule": skip.rule.as_ref(),
                    "path": skip.path.0.as_ref(),
                    "reason": match skip.reason {
                        ryo::linter::SkipReason::SyntaxRejected => "syntax-rejected",
                    },
                })
            })
            .collect::<Vec<_>>()
    });
    let expected = finding_values(&case.expected.diagnostics);
    let mut source_hasher = sha2::Sha256::new();
    for file in &case.request.files {
        source_hasher.update(file.0.as_bytes());
        source_hasher.update([0]);
        source_hasher.update(std::fs::read(case.request.root.join(file.0.as_ref()))?);
    }
    let context_identity = case.request.context_manifest.as_ref().map(|manifest| {
        let hash = crate::outbound::context::adapters::local::adapter::fingerprint(manifest);
        serde_json::json!({ "mode": "listed-disk", "logical_root": manifest.logical_root,
            "actual_root": case.request.root, "manifest_hash": hash.0 })
    });
    let record = serde_json::json!({
        "rule": case.rule.as_ref(),
        "name": case.name.to_string(),
        "side": side,
        "files": files,
        "diagnostics": findings,
        "skipped": skipped,
        "coverage": report.and_then(|report| {
            report.coverage.map(|coverage| {
                serde_json::json!({
                    "selected_rule_file_pairs": coverage.selected,
                    "evaluated_rule_file_pairs": coverage.evaluated,
                    "skipped_rule_file_pairs": coverage.skipped,
                    "unknown_or_unread_rule_file_pairs": coverage.unknown_or_unread,
                })
            })
        }),
        "context_manifest": context_identity,
        "project_context": report.and_then(|report| report.project_context.clone()).map(crate::outbound::context::adapters::local::schemas::ReceiptSchema::from),
        "expected": expected,
        "reference_expected": finding_values(&super::discrepancy::reference_expected(case).diagnostics),
        "documented_reference_discrepancy": case.reference_discrepancy.is_some(),
        "discrepancy_validation": validation,
        "raw_parity_matches_intended": report.map(|actual| super::discrepancy::matches_intended(case, actual)),
        "source_sha256": format!("{:x}", source_hasher.finalize()),
        "error": error,
        "case_passed": passed,
        "complete": true,
    });
    let mut line = serde_json::to_vec(&record)
        .map_err(|serialization| Error::Invalid(serialization.to_string()))?;
    line.push(b'\n');
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
    let mut cases = suite.cases(
        crate::outbound::suite::types::ArtifactPath(rules),
        crate::outbound::suite::types::ArtifactPath(scratch),
    )?;
    super::discrepancy::attach(suite, rules, &mut cases)?;
    let count = cases.len();
    if cases.is_empty() {
        return Err(Error::Invalid(super::constants::NO_CASES.into()));
    }
    let mut failures = Vec::new();
    let validations = super::discrepancy::validate_all(suite, reference, rules, &cases)?;
    for (case, validation) in cases.into_iter().zip(validations) {
        match run(reference, execution, &case.request) {
            Ok(mut result) => {
                let full_report = result.report.clone();
                if case.comparison == crate::domain::suite::types::CaseComparison::Rule {
                    result.report.diagnostics.retain(|finding| {
                        finding.rule == case.rule
                            || finding.rule.as_ref()
                                == crate::domain::diagnostics::constants::FRONTEND_SYNTAX
                    });
                }
                super::discrepancy::complete(&case, &result.report, false)?;
                let comparison = compare(
                    super::discrepancy::reference_expected(&case),
                    &result.report,
                );
                write_case_observation(
                    &case,
                    super::constants::REFERENCE,
                    Some(&full_report),
                    None,
                    comparison.is_ok(),
                    validation.as_ref(),
                )?;
                if let Err(difference) = comparison {
                    failures.push(format!(
                        "{}/{} expected-vs-reference: expected={:?}; actual={:?}",
                        case.rule, case.name, difference.reference, difference.candidate
                    ));
                }
            }
            Err(error) => {
                write_case_observation(
                    &case,
                    super::constants::REFERENCE,
                    None,
                    Some(&error.to_string()),
                    false,
                    validation.as_ref(),
                )?;
                failures.push(format!("{}/{} reference: {error}", case.rule, case.name));
            }
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
        suite,
        reference,
        candidate,
        execution,
        profile,
        scratch,
        CaseSelection::default(),
    )
}

/// Verify the full profile accounting and then execute one deterministic case shard.
///
/// # Errors
/// Refuses invalid profile accounting or case selection and propagates verification failures.
pub fn verify_accounted_shard(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    profile: &Path,
    scratch: &Path,
    selection: CaseSelection<'_>,
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
        selection,
    )
}

/// Serialize an exact diagnostic multiset without changing order or dropping duplicates.
fn finding_values(
    findings: &[crate::domain::diagnostics::types::BenchmarkFinding],
) -> Vec<serde_json::Value> {
    findings
        .iter()
        .map(|finding| {
            serde_json::json!({
                "rule": finding.rule.as_ref(), "file": finding.file.0.as_ref(),
                "start": finding.start.0, "end": finding.end.0,
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "shard_tests.rs"]
mod shard_tests;
