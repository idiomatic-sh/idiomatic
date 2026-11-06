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
            crate::application::run::constants::DECLARED_FILE_OBSERVATIONS_REQUIRE_VERIFICAT.into(),
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
use crate::domain::discrepancy::types::{Contestant, ReferenceProof};
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
                crate::application::run::constants::CASE_SELECTION_IS_EMPTY_OR_OUTSIDE.into(),
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
    crate::application::discrepancy_attachment::service::attach(suite, rules, &mut cases)?;
    let Selection {
        global_count,
        selected_cases,
    } = select_cases(cases, selection)?;
    let count = selected_cases.len();
    let first = selected_cases
        .first()
        .ok_or_else(|| Error::Invalid(super::constants::NO_CASES.into()))?;
    let proofs = crate::application::discrepancy_validation::service::validate_all(
        suite,
        reference,
        rules,
        &selected_cases,
    )?;
    let validations = proofs
        .iter()
        .map(|proof| proof.as_ref().map(proof_value))
        .collect::<Vec<_>>();
    // The normal candidate path loads and compiles the whole selected profile.
    // Reuse its first result; a startup failure must not repeat for every fixture.
    if selected_cases
        .iter()
        .any(|case| crate::domain::process_failure::service::expected(case).is_some())
        && !execution.observes_completed_processes_without_deadline()
    {
        return Err(Error::Invalid(
            crate::application::run::constants::PROCESS_FAILURE_VERIFICATION_REQUIRES_A_DIRE.into(),
        ));
    }
    let mut initial = Some(super::case_execution::candidate(
        suite,
        candidate,
        execution,
        &first.request,
    )?);
    let mut failures = Vec::new();
    for ((case, validation), proof) in selected_cases.into_iter().zip(validations).zip(proofs) {
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
                        && crate::domain::process_failure::service::expected(&case).is_some()
                    {
                        let message = crate::application::run::constants::REFERENCE_RETURNED_A_DECODED_REPORT_INSTEAD;
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
                        crate::domain::discrepancy::service::reference_expected(&case)?
                    } else {
                        &case.expected
                    };
                    crate::domain::discrepancy::service::complete(
                        &case,
                        &result.report,
                        if name == super::constants::CANDIDATE {
                            Contestant::Candidate
                        } else {
                            Contestant::Reference
                        },
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
                        .and(capture.as_ref().zip(proof.as_ref()))
                        .map(|(capture, proof)| {
                            process_matches(&case, capture, proof, suite, rules)
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
    crate::application::discrepancy_attachment::service::attach(suite, rules, &mut cases)?;
    let count = cases.len();
    if cases.is_empty() {
        return Err(Error::Invalid(super::constants::NO_CASES.into()));
    }
    let mut failures = Vec::new();
    let proofs = crate::application::discrepancy_validation::service::validate_all(
        suite, reference, rules, &cases,
    )?;
    let validations = proofs
        .iter()
        .map(|proof| proof.as_ref().map(proof_value))
        .collect::<Vec<_>>();
    if cases
        .iter()
        .any(|case| crate::domain::process_failure::service::expected(case).is_some())
        && !execution.observes_completed_processes_without_deadline()
    {
        return Err(Error::Invalid(
            crate::application::run::constants::PROCESS_FAILURE_VERIFICATION_REQUIRES_A_DIRE.into(),
        ));
    }
    for ((case, validation), proof) in cases.into_iter().zip(validations).zip(proofs) {
        let super::case_execution::Outcome { result, capture } =
            super::case_execution::run(suite, reference, execution, &case, rules);
        match result {
            Ok(mut result) => {
                let full_report = result.report.clone();
                if crate::domain::process_failure::service::expected(&case).is_some() {
                    let message = crate::application::run::constants::REFERENCE_RETURNED_A_DECODED_REPORT_INSTEAD;
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
                crate::domain::discrepancy::service::complete(
                    &case,
                    &result.report,
                    Contestant::Reference,
                )?;
                let comparison = compare(
                    crate::domain::discrepancy::service::reference_expected(&case)?,
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
                let assessment = capture
                    .as_ref()
                    .zip(proof.as_ref())
                    .map(|(capture, proof)| process_matches(&case, capture, proof, suite, rules));
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

/// The proof document the receipt writer records, until the writer takes the typed proof.
fn proof_value(proof: &ReferenceProof) -> serde_json::Value {
    use crate::domain::discrepancy::types::{OptionalDigest, ReferenceKind};
    use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;
    let pinned = |digest: &OptionalDigest| match digest {
        OptionalDigest::Absent => serde_json::Value::Null,
        OptionalDigest::Pinned(digest) => serde_json::json!(digest.0),
    };
    let mut value = serde_json::json!({
        super::constants::DISCREPANCY_BASIS: match proof.basis {
            DiscrepancyBasis::LanguageDefect => super::constants::LANGUAGE_DEFECT,
            DiscrepancyBasis::ToolContract => super::constants::TOOL_CONTRACT_DIFFERENCE,
        },
        super::constants::PROCESS_OBSERVATION_GROUP_SHA256: pinned(&proof.process_group),
        super::constants::REFERENCE_EXPECTATION_KIND: match proof.kind {
            ReferenceKind::ProcessFailure => super::constants::PROCESS_FAILURE,
            ReferenceKind::Diagnostics => super::constants::DIAGNOSTICS,
        },
        super::constants::SETTINGS_SHA256: proof.settings.0,
        super::constants::ACTUAL_EXECUTABLE_SHA256: proof.executable.0,
        super::constants::PROFILE_SHA256: proof.profile.0,
        super::constants::SOURCE_SHA256: proof.source.0,
        super::constants::REFERENCE_OUTPUT_SHA256: proof.reference_output.0,
        super::constants::INTENDED_OUTPUT_SHA256: proof.intended_output.0,
    });
    if let OptionalDigest::Pinned(digest) = &proof.file_metadata {
        value[super::constants::FILE_METADATA_SHA256] = serde_json::json!(digest.0);
    }
    value
}

/// Match the actual reference process against its declared failure, decoding the captured record first.
fn process_matches(
    case: &ryo_benchmark_contracts::domain::workloads::types::PreparedCase,
    capture: &super::case_execution::Capture,
    proof: &ReferenceProof,
    suite: &dyn SuitePort,
    rules: &Path,
) -> Result<(), Error> {
    let record =
        serde_json::to_vec(&capture.invocation).map_err(|error| Error::Json(Box::new(error)))?;
    let document = suite.decode_invocation(&record)?;
    crate::application::process_discrepancy::service::matches(
        case,
        &document,
        &capture.output,
        proof,
        suite,
        rules,
    )?;
    Ok(())
}
