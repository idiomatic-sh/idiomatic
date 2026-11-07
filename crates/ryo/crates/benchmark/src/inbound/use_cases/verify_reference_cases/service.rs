//! The reference of every admitted case answers its own derived expectations.
use ryo_benchmark_contracts::domain::documents::types::text::value::{ReasonText, YesNo};
use crate::application::case_execution::{service::run as run_case, types::AttemptOutcome};
use crate::application::case_observation::service::write as record;
use crate::domain::discrepancy::service::judge;
use crate::domain::discrepancy::types::{Contestant, ReferenceProof};
use crate::domain::process_failure::types::CaptureRecord;
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
use ryo_benchmark_contracts::domain::workloads::types::{
    PreparedCase, reference_case_count::value::ReferenceCaseCount,
};
use std::path::Path;

/// Match the actual reference process against its declared failure, when one was captured.
fn process_matches(
    case: &PreparedCase,
    capture: &CaptureRecord,
    proof: Option<&ReferenceProof>,
    suite: &dyn SuitePort,
    rules: &Path,
) -> Option<Result<(), Error>> {
    let (CaptureRecord::Kept(capture), Some(proof)) = (capture, proof) else {
        return None;
    };
    Some(
        crate::application::process_discrepancy::service::matches(
            case,
            &capture.invocation,
            &capture.output,
            proof,
            suite,
            rules,
        )
        .map_err(Error::from),
    )
}

/// Execute admitted cases through the actual reference-only verification workflow.
pub(super) fn verify_prepared_reference_cases(
    suite: &dyn SuitePort,
    receipts: &dyn ReceiptsPort,
    context: &dyn ContextPort,
    reference: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    inventory: &dyn InventoryPort,
    rules: &Path,
    mut cases: Vec<PreparedCase>,
) -> Result<ReferenceCaseCount, Error> {
    if cases.is_empty() {
        return Err(Error::Invalid(constants::NO_CASES.into()));
    }
    crate::application::input_metadata::service::admit(suite, rules, &cases)?;
    crate::application::discrepancy_attachment::service::attach(suite, rules, &mut cases)?;
    let count = cases.len();
    let mut failures = Vec::new();
    let proofs = crate::application::discrepancy_validation::service::validate_all(
        suite, inventory, reference, rules, &cases,
    )?;
    if cases
        .iter()
        .any(|case| crate::domain::process_failure::service::expected(case).is_some())
        && execution.exit_observation() != ExitObservationMode::Direct
    {
        return Err(Error::Invalid(
            constants::PROCESS_FAILURE_VERIFICATION_REQUIRES_A_DIRE.into(),
        ));
    }
    let contestant = Contestant::Reference;
    for (case, proof) in cases.into_iter().zip(proofs) {
        let AttemptOutcome { result, capture } = run_case(suite, context, reference, execution, &case, rules);
        match result {
            Ok(result) => {
                let full_report = result.report.clone();
                if crate::domain::process_failure::service::expected(&case).is_some() {
                    let message =
                        ReasonText(constants::REFERENCE_RETURNED_A_DECODED_REPORT_INSTEAD.into());
                    record(
                        suite,
                        receipts,
                        context,
                        &case,
                        contestant,
                        Some(&full_report),
                        Some(&message),
                        YesNo::No,
                        proof.as_ref(),
                        &CaptureRecord::Absent,
                    )?;
                    failures.push(format!("{}/{} {contestant}: {}", case.rule, case.name, message.0));
                    continue;
                }
                let comparison = judge(&case, contestant, result.report)?.comparison;
                record(
                    suite,
                    receipts,
                    context,
                    &case,
                    contestant,
                    Some(&full_report),
                    None,
                    YesNo::from(comparison.is_ok()),
                    proof.as_ref(),
                    &capture,
                )?;
                if let Err(difference) = comparison {
                    failures.push(format!(
                        "{}/{} expected-vs-{contestant}: expected={:?}; actual={:?}",
                        case.rule, case.name, difference.reference, difference.candidate
                    ));
                }
            }
            Err(error) => {
                let assessment = process_matches(&case, &capture, proof.as_ref(), suite, rules);
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
                    &capture,
                )?;
                if !accepted {
                    failures.push(format!("{}/{} {contestant}: {}", case.rule, case.name, message.0));
                }
            }
        }
    }
    if failures.is_empty() {
        Ok(ReferenceCaseCount(count))
    } else {
        Err(Error::Invalid(format!(
            "{count} reference-only cases checked; failures:\n{}",
            failures.join(constants::LINE_SEPARATOR)
        )))
    }
}

