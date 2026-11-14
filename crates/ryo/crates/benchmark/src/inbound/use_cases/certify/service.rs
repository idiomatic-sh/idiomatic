//! A complete disposition inventory is distinct from verified subset coverage.
use super::types::RegisteredContext;
use crate::application::case_execution::{
    service::{candidate as run_candidate, run as run_case},
    types::AttemptOutcome,
};
use crate::application::case_observation::service::write as record;
use crate::application::case_selection::{service::select, types::ShardSelection};
use crate::domain::discrepancy::service::{describe_difference, judge};
use crate::domain::discrepancy::types::Contestant;
use crate::domain::process_failure::types::CaptureRecord;
use crate::domain::suite::constants;
use crate::outbound::{
    context::interface::ContextPort,
    execution::{interface::ExecutionPort, types::ExitObservationMode},
    inventory::{interface::InventoryPort, types::Registration},
    linter::interface::LinterPort,
    receipts::interface::ReceiptsPort,
    suite::interface::SuitePort,
};
use ryo_benchmark_contracts::domain::documents::types::text::value::{ReasonText, YesNo};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::CaseSelection;
use ryo_benchmark_contracts::domain::workloads::types::{
    PreparedCase, Tool, case_count::value::CaseCount,
};
use std::path::{Path, PathBuf};

/// Preserve every registered context, and require measured code to be the code actually tested.
pub(super) fn prepare(
    store: &dyn InventoryPort,
    suite: &dyn SuitePort,
    registration: &Path,
    measured_profile: &Path,
    tool: &Tool,
    rules: &Path,
    reference: &Path,
    candidate: &Path,
    coverage: &ryo_benchmark_contracts::domain::requests::types::RegisteredCoverage,
) -> Result<RegisteredContext, Error> {
    let loaded = store.load(registration)?;
    crate::application::inventory_verification::authority::service::require_authority(
        &loaded, suite, store,
    )?;
    let mut paths = crate::application::registered_certification::service::admit(
        &loaded,
        suite,
        registration,
        measured_profile,
        tool,
        rules,
        reference,
        candidate,
        coverage,
    )?;
    for entry in
        crate::domain::suite::service::covered(coverage, &loaded.rules, |entry| &entry.rule_id)
    {
        crate::application::inventory_verification::authority::service::verify_context(
            entry, suite, store,
        )
        .map_err(|reason| Error::Invalid(reason.0))?;
        paths.extend(
            crate::application::registered_certification::service::pin_rule(
                entry,
                suite,
                measured_profile,
                tool,
                rules,
            )?,
        );
    }
    paths.sort();
    paths.dedup();
    Ok(RegisteredContext {
        registration: loaded,
        paths,
    })
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
    selection: CaseSelection,
) -> Result<CaseCount, Error> {
    if cases.is_empty() {
        return Err(Error::Invalid(constants::NO_CASES.into()));
    }
    crate::application::input_metadata::service::admit(suite, rules, &cases)?;
    crate::application::discrepancy_attachment::service::attach(suite, rules, &mut cases)?;
    let ShardSelection {
        global_count,
        selected_cases,
    } = select(cases, selection);
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
                            &CaptureRecord::Absent,
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
                        &capture,
                    )?;
                    if let Err(difference) = comparison {
                        failures.push(describe_difference(&case, contestant, &difference).0);
                    }
                }
                Err(error) => {
                    let assessment = (contestant == Contestant::Reference)
                        .then(|| {
                            crate::application::process_discrepancy::service::matches_capture(
                                &case,
                                &capture,
                                proof.as_ref(),
                                suite,
                                rules,
                            )
                        })
                        .flatten();
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
