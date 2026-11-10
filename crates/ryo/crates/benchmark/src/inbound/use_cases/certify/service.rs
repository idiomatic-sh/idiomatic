//! A complete disposition inventory is distinct from verified subset coverage.
use super::types::{PreparedCases, RegisteredContext};
use crate::application::case_execution::{
    service::{candidate as run_candidate, run as run_case},
    types::AttemptOutcome,
};
use crate::application::case_observation::service::write as record;
use crate::application::case_selection::{service::select, types::ShardSelection};
use crate::domain::discrepancy::service::{describe_difference, judge};
use crate::domain::discrepancy::types::Contestant;
use crate::domain::process_failure::types::CaptureRecord;
use crate::domain::suite::{
    constants,
    service::require_accounted,
    types::{audit::value::Audit, certificate::value::Certificate},
};
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

/// Execute independent cases, then seal the unchanged artifacts and both executable bytes.
#[expect(
    clippy::too_many_arguments,
    reason = "this orchestration forwards explicit sealed workload identities through four independently injected ports"
)]
pub(super) fn certify(
    suite: &dyn SuitePort,
    receipts: &dyn ReceiptsPort,
    context: &dyn ContextPort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    inventory_store: &dyn InventoryPort,
    directory: &Path,
    workload: &Path,
    reference_executable: &Path,
    candidate_executable: &Path,
    selected_rules: &Path,
    scratch: &Path,
    scope: ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage,
    case_source: &ryo_benchmark_contracts::domain::requests::types::CertificationCases,
) -> Result<Certificate, Error> {
    let tool = suite.profile(crate::outbound::suite::types::ArtifactPath(directory))?;
    suite.verify_tool(
        crate::outbound::suite::types::ArtifactPath(reference_executable),
        &tool,
    )?;
    let audit = crate::application::suite_auditing::service::audit(suite, directory, &tool)?;
    if scope == ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage::Accounted {
        require_accounted(&audit, &tool)?;
    }
    if audit.implemented.is_empty() {
        return Err(Error::Invalid(
            crate::domain::suite::constants::EMPTY_SUBSET.into(),
        ));
    }
    let contestants = [
        reference_executable.to_owned(),
        candidate_executable.to_owned(),
    ];
    let execution_profile = execution.profile();
    let mut paths = vec![
        directory.to_owned(),
        workload.to_owned(),
        selected_rules.to_owned(),
        contestants[0].clone(),
        contestants[1].clone(),
    ];
    if let ryo_benchmark_contracts::domain::measurements::types::ExecutionProfile::Isolated(
        collector,
    ) = &execution_profile
    {
        paths.push(collector.executable.clone());
    }
    let prepared_cases = match case_source {
        ryo_benchmark_contracts::domain::requests::types::CertificationCases::Profile => {
            PreparedCases::Profile
        }
        ryo_benchmark_contracts::domain::requests::types::CertificationCases::Registered {
            registration,
            output,
        } => {
            let RegisteredContext {
                registration,
                paths: context_paths,
            } = prepare(
                inventory_store,
                suite,
                registration,
                directory,
                &tool,
                selected_rules,
                reference_executable,
                candidate_executable,
            )?;
            paths.extend(context_paths);
            PreparedCases::Registered {
                registration,
                output: output.clone(),
            }
        }
    };
    let before = suite.fingerprint(crate::outbound::suite::types::ArtifactSet(&paths))?;
    let crate::application::certification_probe::types::ProbedProfiles { probe, profiles } =
        crate::application::certification_probe::service::probe(
            reference, candidate, directory, scratch,
        )?;
    let cases = match &prepared_cases {
        PreparedCases::Profile => {
            let rules = directory.join(super::constants::RULE_DIRECTORY);
            let prepared = crate::application::case_preparation::service::cases(
                suite, context, &rules, scratch,
            )?;
            verify_prepared_shard(
                suite,
                receipts,
                context,
                reference,
                candidate,
                execution,
                inventory_store,
                &rules,
                prepared,
                ryo_benchmark_contracts::domain::requests::types::CaseSelection::default(),
            )?
        }
        PreparedCases::Registered {
            registration,
            output,
        } => crate::application::inventory_verification::service::certify_registered(
            registration,
            output,
            suite,
            receipts,
            execution,
            inventory_store,
        )?,
    };
    let after = suite.fingerprint(crate::outbound::suite::types::ArtifactSet(&paths))?;
    if before != after || execution.profile() != execution_profile {
        return Err(Error::Invalid(super::constants::CHANGED_EVIDENCE.into()));
    }
    if let ryo_benchmark_contracts::domain::requests::types::CertificationCases::Registered {
        output,
        ..
    } = case_source
    {
        paths.push(output.clone());
    }
    let sealed = suite.fingerprint(crate::outbound::suite::types::ArtifactSet(&paths))?;
    Ok(Certificate {
        coverage: Audit {
            implemented: audit.implemented,
            missing: Vec::new(),
        },
        paths,
        fingerprint: sealed,
        contestants,
        probe,
        profiles,
        execution: execution_profile,
        cases,
        case_source: case_source.clone(),
    })
}

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
) -> Result<RegisteredContext, Error> {
    let loaded = store.load(registration)?;
    crate::application::inventory_verification::service::require_authority(&loaded, suite, store)?;
    let mut paths = crate::application::registered_certification::service::admit(
        &loaded,
        suite,
        registration,
        measured_profile,
        tool,
        rules,
        reference,
        candidate,
    )?;
    for entry in &loaded.rules {
        crate::application::inventory_verification::service::verify_context(entry, suite, store)
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
fn verify_prepared_shard(
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
