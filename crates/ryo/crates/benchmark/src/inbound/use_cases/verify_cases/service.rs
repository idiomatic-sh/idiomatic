//! The reference and candidate of every selected case answer the same expectations.
use ryo_benchmark_contracts::domain::documents::types::text::value::{ReasonText, YesNo};
use crate::application::case_execution::{
    service::{candidate as run_candidate, run as run_case},
    types::{AttemptOutcome, CaptureRecord, ProcessCapture},
};
use crate::application::case_observation::{
    service::write as record,
    types::{ObservedProcess, ProcessEvidence},
};
use crate::application::case_selection::{service::select, types::ShardSelection};
use crate::domain::discrepancy::service::judge;
use crate::domain::discrepancy::types::{Contestant, ReferenceProof};
use crate::domain::suite::constants;
use crate::outbound::receipts::interface::ReceiptsPort;
use crate::outbound::{
    context::interface::ContextPort,
    execution::{interface::ExecutionPort, types::ExitObservationMode},
    inventory::interface::InventoryPort,
    linter::interface::LinterPort,
    suite::interface::SuitePort,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::CaseSelection;
use ryo_benchmark_contracts::domain::workloads::types::{
    PreparedCase, case_count::value::CaseCount,
};
use std::path::Path;

/// Match the actual reference process against its declared failure.
fn process_matches(
    case: &PreparedCase,
    capture: &ProcessCapture,
    proof: &ReferenceProof,
    suite: &dyn SuitePort,
    rules: &Path,
) -> Result<(), Error> {
    crate::application::process_discrepancy::service::matches(
        case,
        &capture.invocation,
        &capture.output,
        proof,
        suite,
        rules,
    )?;
    Ok(())
}

/// Execute admitted materialized cases through the same verification workflow.
#[expect(
    clippy::too_many_arguments,
    reason = "the workflow forwards the six independently injected ports and the admitted cases"
)]
pub(super) fn verify_prepared_shard(
    suite: &dyn SuitePort,
    receipts: &dyn ReceiptsPort,
    context: &dyn ContextPort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    inventory: &dyn InventoryPort,
    rules: &Path,
    mut cases: Vec<PreparedCase>,
    selection: CaseSelection<'_>,
) -> Result<CaseCount, Error> {
    if cases.is_empty() {
        return Err(Error::Invalid(constants::NO_CASES.into()));
    }
    crate::application::input_metadata::service::admit(suite, rules, &cases)?;
    crate::application::discrepancy_attachment::service::attach(suite, rules, &mut cases)?;
    let ShardSelection {
        global_count,
        selected_cases,
    } = select(cases, selection)?;
    let count = selected_cases.len();
    let first = selected_cases
        .first()
        .ok_or_else(|| Error::Invalid(constants::NO_CASES.into()))?;
    let proofs = crate::application::discrepancy_validation::service::validate_all(
        suite,
        inventory,
        reference,
        rules,
        &selected_cases,
    )?;
    // The normal candidate path loads and compiles the whole selected profile.
    // Reuse its first result; a startup failure must not repeat for every fixture.
    if selected_cases
        .iter()
        .any(|case| crate::domain::process_failure::service::expected(case).is_some())
        && execution.exit_observation() != ExitObservationMode::Direct
    {
        return Err(Error::Invalid(
            constants::PROCESS_FAILURE_VERIFICATION_REQUIRES_A_DIRE.into(),
        ));
    }
    let mut initial = Some(run_candidate(
        suite,
        context,
        candidate,
        execution,
        &first.request,
    )?);
    let mut failures = Vec::new();
    for (case, proof) in selected_cases.into_iter().zip(proofs) {
        for (contestant, tool) in [
            (Contestant::Reference, reference),
            (Contestant::Candidate, candidate),
        ] {
            let AttemptOutcome { result, capture } = if contestant == Contestant::Candidate {
                AttemptOutcome {
                    result: initial.take().map_or_else(
                        || run_candidate(suite, context, tool, execution, &case.request),
                        Ok,
                    ),
                    capture: CaptureRecord::Absent,
                }
            } else {
                run_case(suite, context, tool, execution, &case, rules)
            };
            let full_report = match &result {
                Ok(measurement) => Some(measurement.report.clone()),
                Err(_failed) => None,
            };
            match result {
                Ok(result) => {
                    if contestant == Contestant::Reference
                        && crate::domain::process_failure::service::expected(&case).is_some()
                    {
                        let message = ReasonText(
                            constants::REFERENCE_RETURNED_A_DECODED_REPORT_INSTEAD.into(),
                        );
                        record(
                            suite,
                            receipts,
                            context,
                            &case,
                            contestant,
                            full_report.as_ref(),
                            Some(&message),
                            YesNo::No,
                            proof.as_ref(),
                            ProcessEvidence::Absent,
                        )?;
                        failures.push(format!(
                            "{}/{} {contestant}: {}",
                            case.rule, case.name, message.0
                        ));
                        continue;
                    }
                    let comparison = judge(&case, contestant, result.report)?.comparison;
                    record(
                        suite,
                        receipts,
                        context,
                        &case,
                        contestant,
                        full_report.as_ref(),
                        None,
                        YesNo::from(comparison.is_ok()),
                        proof.as_ref(),
                        evidence(&capture),
                    )?;
                    if let Err(difference) = comparison {
                        failures.push(format!(
                            "{}/{} expected-vs-{contestant}: expected={:?}; actual={:?}; expected-skipped={:?}; actual-skipped={:?}",
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
                    let assessment = (contestant == Contestant::Reference)
                        .then_some(())
                        .and(kept(&capture).zip(proof.as_ref()))
                        .map(|(capture, proof)| process_matches(&case, capture, proof, suite, rules));
                    let accepted = matches!(assessment, Some(Ok(())));
                    let message = ReasonText(match assessment {
                        Some(Err(mismatch)) => format!("{error}; {mismatch}"),
                        _ => error.to_string(),
                    });
                    record(
                        suite,
                        receipts,
                        context,
                        &case,
                        contestant,
                        None,
                        if accepted { None } else { Some(&message) },
                        YesNo::from(accepted),
                        proof.as_ref(),
                        evidence(&capture),
                    )?;
                    if !accepted {
                        failures.push(format!(
                            "{}/{} {contestant}: {}",
                            case.rule, case.name, message.0
                        ));
                    }
                }
            }
        }
    }
    if failures.is_empty() {
        debug_assert!(
            count <= global_count.0,
            "selected cases must belong to the audited inventory"
        );
        Ok(CaseCount(count))
    } else {
        Err(Error::Invalid(format!(
            "{count} cases checked; failures:\n{}",
            failures.join(constants::LINE_SEPARATOR)
        )))
    }
}

/// The raw process a captured attempt keeps beside its observation row.
fn evidence(capture: &CaptureRecord) -> ProcessEvidence<'_> {
    match capture {
        CaptureRecord::Absent => ProcessEvidence::Absent,
        CaptureRecord::Kept(kept) => ProcessEvidence::Kept(ObservedProcess {
            invocation: &kept.invocation,
            output: &kept.output,
        }),
    }
}

/// The captured attempt, when its raw bytes were kept.
fn kept(capture: &CaptureRecord) -> Option<&ProcessCapture> {
    match capture {
        CaptureRecord::Absent => None,
        CaptureRecord::Kept(kept) => Some(kept),
    }
}
