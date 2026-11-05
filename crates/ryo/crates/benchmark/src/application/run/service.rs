//! Preparation and decoding are deliberately outside the measured interval.
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::linter::interface::LinterPort;
use ryo_benchmark_contracts::domain::measurements::types::Measurement;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;

/// Prepare a measurement request with no declared file observations, then validate decoded coverage.
/// Observation-bearing verification uses the suite-owned `case_execution::prepare` boundary.
///
/// # Errors
/// Propagates preparation, launch or decoding errors and refuses invalid input coverage or ranges.
pub fn run(
    linter: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    request: &RunRequest,
) -> Result<Measurement, Error> {
    if request.file_metadata.is_some() {
        return Err(Error::Invalid(
            "declared file observations require verification preparation".into(),
        ));
    }
    if let Some(context) = &request.context_manifest {
        crate::outbound::context::adapters::local::adapter::validate(context, &request.root)?;
    }
    super::case_execution::configuration(linter, request)?;
    let invocation = linter.prepare(request)?;
    let output = execution.run(invocation)?;
    finish(linter, request, output)
}

/// Validate a captured process through the existing adapter and input-coverage contract.
pub(super) fn finish(
    linter: &dyn LinterPort,
    request: &RunRequest,
    output: ryo_benchmark_contracts::domain::measurements::types::ProcessOutput,
) -> Result<Measurement, Error> {
    let wall = output.wall;
    let resources = output.resources;
    let report = linter.decode(request, output)?;
    validate_report(request, &report)?;
    Ok(Measurement {
        wall,
        resources,
        report,
    })
}

/// Validate decoded coverage without treating a rejected report as a process failure.
pub(super) fn validate_report(request: &RunRequest, report: &RunReport) -> Result<(), Error> {
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
    Ok(())
}

use crate::domain::diagnostics::service::compare_diagnostics as compare;
use crate::outbound::suite::interface::SuitePort;
use ryo_benchmark_contracts::domain::requests::types::CaseSelection;
use std::path::Path;
/// Audited global inventory and its exact selected partition.
struct Selection {
    /// Number of cases before partitioning.
    global_count: usize,
    /// Cases admitted to this invocation, preserving deterministic order.
    selected_cases: Vec<ryo_benchmark_contracts::domain::workloads::types::PreparedCase>,
}

/// Select an exact, nonempty subset only after the complete inventory is loaded.
fn select_cases(
    mut cases: Vec<ryo_benchmark_contracts::domain::workloads::types::PreparedCase>,
    selection: CaseSelection<'_>,
) -> Result<Selection, Error> {
    cases.sort_by(|left, right| (&left.rule, &left.name).cmp(&(&right.rule, &right.name)));
    let global_count = cases.len();
    let assigned = match selection.shard {
        Some(shard) => cases
            .into_iter()
            .enumerate()
            .filter_map(|(index, case)| (index % shard.count() == shard.index()).then_some(case))
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

/// Execute admitted materialized cases through the same verification workflow.
pub(crate) fn verify_prepared_shard(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    rules: &Path,
    mut cases: Vec<ryo_benchmark_contracts::domain::workloads::types::PreparedCase>,
    selection: CaseSelection<'_>,
) -> Result<ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount, Error>
{
    if cases.is_empty() {
        return Err(Error::Invalid(super::constants::NO_CASES.into()));
    }
    super::input_metadata::admit(suite, rules, &cases)?;
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
    if selected_cases
        .iter()
        .any(|case| super::process_discrepancy::expected(case).is_some())
        && !execution.observes_completed_processes_without_deadline()
    {
        return Err(Error::Invalid(
            "process-failure verification requires a direct no-deadline observer".into(),
        ));
    }
    let mut initial = Some(super::case_execution::candidate(
        suite,
        candidate,
        execution,
        &first.request,
    )?);
    let mut failures = Vec::new();
    for (case, validation) in selected_cases.into_iter().zip(validations) {
        for (name, tool) in [
            (super::constants::REFERENCE, reference),
            (super::constants::CANDIDATE, candidate),
        ] {
            let super::case_execution::Outcome { result, capture } = if name
                == super::constants::CANDIDATE
            {
                super::case_execution::Outcome {
                    result: initial.take().map_or_else(
                        || super::case_execution::candidate(suite, tool, execution, &case.request),
                        Ok,
                    ),
                    capture: None,
                }
            } else {
                super::case_execution::run(suite, tool, execution, &case, rules)
            };
            let full_report = result
                .as_ref()
                .ok()
                .map(|measurement| measurement.report.clone());
            match result {
                Ok(mut result) => {
                    if name == super::constants::REFERENCE
                        && super::process_discrepancy::expected(&case).is_some()
                    {
                        let message = "reference returned a decoded report instead of the declared process failure";
                        super::case_observations::write(
                            suite,
                            &case,
                            name,
                            full_report.as_ref(),
                            Some(message),
                            false,
                            validation.as_ref(),
                            None,
                        )?;
                        failures.push(format!("{}/{} {name}: {message}", case.rule, case.name));
                        continue;
                    }
                    if name == super::constants::CANDIDATE {
                        ryo_benchmark_contracts::domain::reports::service::require_complete_execution(
                            &result.report,
                        )?;
                    }
                    if case.comparison
                        == ryo_benchmark_contracts::domain::workloads::types::CaseComparison::Rule
                    {
                        result.report.diagnostics.retain(|finding| {
                            finding.rule == case.rule
                                || finding.rule.as_ref()
                                    == crate::domain::diagnostics::constants::FRONTEND_SYNTAX
                        });
                    }
                    let expected = if name == super::constants::REFERENCE {
                        super::discrepancy::reference_expected(&case)?
                    } else {
                        &case.expected
                    };
                    super::discrepancy::complete(
                        &case,
                        &result.report,
                        name == super::constants::CANDIDATE,
                    )?;
                    let comparison = compare(expected, &result.report);
                    super::case_observations::write(
                        suite,
                        &case,
                        name,
                        full_report.as_ref(),
                        None,
                        comparison.is_ok(),
                        validation.as_ref(),
                        capture.as_ref(),
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
                    let assessment = (name == super::constants::REFERENCE)
                        .then_some(())
                        .and(capture.as_ref().zip(validation.as_ref()))
                        .map(|(capture, proof)| {
                            super::process_discrepancy::matches(&case, capture, proof, suite, rules)
                        });
                    let accepted = matches!(assessment, Some(Ok(())));
                    let message = match assessment {
                        Some(Err(mismatch)) => format!("{error}; {mismatch}"),
                        _ => error.to_string(),
                    };
                    super::case_observations::write(
                        suite,
                        &case,
                        name,
                        None,
                        if accepted { None } else { Some(&message) },
                        accepted,
                        validation.as_ref(),
                        capture.as_ref(),
                    )?;
                    if !accepted {
                        failures.push(format!("{}/{} {name}: {message}", case.rule, case.name));
                    }
                }
            }
        }
    }
    if failures.is_empty() {
        debug_assert!(
            count <= global_count,
            "selected cases must belong to the audited inventory"
        );
        Ok(ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount(count))
    } else {
        Err(Error::Invalid(format!(
            "{count} cases checked; failures:\n{}",
            failures.join(super::constants::LINE_SEPARATOR)
        )))
    }
}

/// Execute admitted cases through the actual reference-only verification workflow.
pub(crate) fn verify_prepared_reference_cases(
    suite: &dyn SuitePort, reference: &dyn LinterPort, execution: &dyn ExecutionPort,
    rules: &Path, mut cases: Vec<ryo_benchmark_contracts::domain::workloads::types::PreparedCase>,
) -> Result<ryo_benchmark_contracts::domain::workloads::types::reference_case_count::value::ReferenceCaseCount, Error>{
    if cases.is_empty() {
        return Err(Error::Invalid(super::constants::NO_CASES.into()));
    }
    super::input_metadata::admit(suite, rules, &cases)?;
    super::discrepancy::attach(suite, rules, &mut cases)?;
    let count = cases.len();
    if cases.is_empty() {
        return Err(Error::Invalid(super::constants::NO_CASES.into()));
    }
    let mut failures = Vec::new();
    let validations = super::discrepancy::validate_all(suite, reference, rules, &cases)?;
    if cases
        .iter()
        .any(|case| super::process_discrepancy::expected(case).is_some())
        && !execution.observes_completed_processes_without_deadline()
    {
        return Err(Error::Invalid(
            "process-failure verification requires a direct no-deadline observer".into(),
        ));
    }
    for (case, validation) in cases.into_iter().zip(validations) {
        let super::case_execution::Outcome { result, capture } =
            super::case_execution::run(suite, reference, execution, &case, rules);
        match result {
            Ok(mut result) => {
                let full_report = result.report.clone();
                if super::process_discrepancy::expected(&case).is_some() {
                    let message = "reference returned a decoded report instead of the declared process failure";
                    super::case_observations::write(
                        suite,
                        &case,
                        super::constants::REFERENCE,
                        Some(&full_report),
                        Some(message),
                        false,
                        validation.as_ref(),
                        None,
                    )?;
                    failures.push(format!("{}/{} reference: {message}", case.rule, case.name));
                    continue;
                }
                if case.comparison
                    == ryo_benchmark_contracts::domain::workloads::types::CaseComparison::Rule
                {
                    result.report.diagnostics.retain(|finding| {
                        finding.rule == case.rule
                            || finding.rule.as_ref()
                                == crate::domain::diagnostics::constants::FRONTEND_SYNTAX
                    });
                }
                super::discrepancy::complete(&case, &result.report, false)?;
                let comparison = compare(
                    super::discrepancy::reference_expected(&case)?,
                    &result.report,
                );
                super::case_observations::write(
                    suite,
                    &case,
                    super::constants::REFERENCE,
                    Some(&full_report),
                    None,
                    comparison.is_ok(),
                    validation.as_ref(),
                    capture.as_ref(),
                )?;
                if let Err(difference) = comparison {
                    failures.push(format!(
                        "{}/{} expected-vs-reference: expected={:?}; actual={:?}",
                        case.rule, case.name, difference.reference, difference.candidate
                    ));
                }
            }
            Err(error) => {
                let assessment =
                    capture
                        .as_ref()
                        .zip(validation.as_ref())
                        .map(|(capture, proof)| {
                            super::process_discrepancy::matches(&case, capture, proof, suite, rules)
                        });
                let accepted = matches!(assessment, Some(Ok(())));
                let message = match assessment {
                    Some(Err(mismatch)) => format!("{error}; {mismatch}"),
                    _ => error.to_string(),
                };
                super::case_observations::write(
                    suite,
                    &case,
                    super::constants::REFERENCE,
                    None,
                    if accepted { None } else { Some(&message) },
                    accepted,
                    validation.as_ref(),
                    capture.as_ref(),
                )?;
                if !accepted {
                    failures.push(format!("{}/{} reference: {message}", case.rule, case.name));
                }
            }
        }
    }
    if failures.is_empty() {
        Ok(ryo_benchmark_contracts::domain::workloads::types::reference_case_count::value::ReferenceCaseCount(count))
    } else {
        Err(Error::Invalid(format!(
            "{count} reference-only cases checked; failures:\n{}",
            failures.join(super::constants::LINE_SEPARATOR)
        )))
    }
}

/// Serialize an exact diagnostic multiset without changing order or dropping duplicates.
pub(super) fn finding_values(
    findings: &[ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding],
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
