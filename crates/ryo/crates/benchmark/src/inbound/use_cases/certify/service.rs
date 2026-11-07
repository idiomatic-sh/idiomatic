//! A complete disposition inventory is distinct from verified subset coverage.
use ryo_benchmark_contracts::domain::documents::types::text::value::{ReasonText, YesNo};
use crate::domain::suite::service::require_accounted;
use crate::domain::suite::types::audit::value::Audit;
use crate::domain::suite::types::certificate::value::Certificate;
use crate::outbound::{
    execution::interface::ExecutionPort, linter::interface::LinterPort, suite::interface::SuitePort,
};
use super::types::{PreparedCases, RegisteredContext};
use crate::outbound::{
    inventory::{
        interface::InventoryPort,
        types::{Registration, RegistrationAuthority},
    },
    suite::types::ArtifactPath,
};
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
    execution::types::ExitObservationMode,
};
use ryo_benchmark_contracts::domain::requests::types::CaseSelection;
use ryo_benchmark_contracts::domain::workloads::types::{PreparedCase};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::Tool;
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
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
    let probe = ryo_benchmark_contracts::domain::requests::types::RunRequest {
        performance: Default::default(),
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Frozen,
        rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        staged_sources: ryo_benchmark_contracts::domain::requests::types::StagedSources::Unlisted,
        file_metadata: ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared,
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
        root: directory.to_owned(),
        files: vec![ryo::linter::RelativePath(
            super::constants::PROFILE_PROBE.into(),
        )],
        state: scratch.join(super::constants::IDENTITY_STATE),
    };
    let profiles = [reference.prepare(&probe)?, candidate.prepare(&probe)?];
    let cases = match &prepared_cases {
        PreparedCases::Profile => {
            let rules = directory.join(super::constants::RULE_DIRECTORY);
            let prepared =
                crate::application::case_preparation::service::cases(suite, context, &rules, scratch)?;
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
        } => execute(registration, output, execution, inventory_store)?,
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
    let loaded = store.load(registration).map_err(Error::Invalid)?;
    crate::application::inventory_verification::service::require_authority(&loaded, store)?;
    let expected: std::collections::BTreeSet<_> =
        tool.defaults.iter().map(ToString::to_string).collect();
    let actual: std::collections::BTreeSet<_> = loaded.defaults.iter().cloned().collect();
    if expected != actual || loaded.defaults.len() != expected.len() {
        return Err(Error::Invalid(
            super::constants::CASE_INVENTORY_DIFFERS_FROM_MEASURED_COMPLET.into(),
        ));
    }
    let RegistrationAuthority::Execution {
        profile,
        reference: reference_pin,
        worker,
        profile_sha256,
    } = &loaded.authority
    else {
        return Err(Error::Invalid(
            super::constants::REGISTERED_CERTIFICATION_REQUIRES_ACTUAL_EXE.into(),
        ));
    };
    if suite
        .digest(ArtifactPath(
            &measured_profile.join(super::constants::FIXTURE_PROFILE_FILE),
        ))?
        .0
        != *profile_sha256
    {
        return Err(Error::Invalid(
            super::constants::REGISTERED_AUTHORITY_DIFFERS_FROM_MEASURED_C.into(),
        ));
    }
    if rules
        != measured_profile
            .join(super::constants::FIXTURE_RULE_DIRECTORY_NAME)
            .as_path()
    {
        return Err(Error::Invalid(
            super::constants::SELECTED_RULE_ROOT_DIFFERS_FROM_MEASURED.into(),
        ));
    }
    if suite.digest(ArtifactPath(reference))?.0 != reference_pin.sha256
        || suite.digest(ArtifactPath(candidate))?.0 != worker.sha256
    {
        return Err(Error::Invalid(
            super::constants::CASE_INVENTORY_EXECUTABLE_IDENTITIES_DIFFER.into(),
        ));
    }
    let mut paths = vec![
        registration.to_owned(),
        profile.join(super::constants::FIXTURE_PROFILE_FILE),
        reference_pin.path.clone(),
        worker.path.clone(),
    ];
    for entry in &loaded.rules {
        if entry.reason.is_some() || entry.command.is_none() {
            return Err(Error::Invalid(format!(
                "unexecutable registered context: {}",
                entry.rule_id
            )));
        }
        let crate::outbound::inventory::types::Command::CaseVerification { context } = entry
            .command
            .as_ref()
            .ok_or_else(|| Error::Invalid(super::constants::MISSING_VERIFICATION_COMMAND.into()))?;
        crate::application::inventory_verification::service::verify_context(entry, store)
            .map_err(Error::Invalid)?;
        let pin = entry.definition.as_ref().ok_or_else(|| {
            Error::Invalid(super::constants::REGISTERED_CONTEXT_LACKS_TESTED_QUERY.into())
        })?;
        if pin.path
            != context
                .join(super::constants::FIXTURE_RULE_DIRECTORY_NAME)
                .join(&entry.rule_id)
                .join(super::constants::QUERY_RYOQL)
            || entry.artifacts.get(&pin.path) != Some(&pin.sha256)
            || suite
                .digest(ArtifactPath(
                    &rules.join(&entry.rule_id).join(super::constants::QUERY_RYOQL),
                ))?
                .0
                != pin.sha256
        {
            return Err(Error::Invalid(format!(
                "measured query differs from registered context: {}",
                entry.rule_id
            )));
        }
        for (name, pin) in &entry.modules {
            let relative = tool.query_modules.get(name).ok_or_else(|| {
                Error::Invalid(format!("measured profile lacks tested module: {name}"))
            })?;
            if entry.artifacts.get(&pin.path) != Some(&pin.sha256)
                || suite
                    .digest(ArtifactPath(&measured_profile.join(relative)))?
                    .0
                    != pin.sha256
            {
                return Err(Error::Invalid(format!(
                    "measured helper differs from tested context: {name}"
                )));
            }
        }
        paths.extend(entry.artifacts.keys().cloned());
    }
    paths.sort();
    paths.dedup();
    Ok(RegisteredContext {
        registration: loaded,
        paths,
    })
}
/// Certify once; cold/warm/edit measurements reuse the sealed certificate.
fn execute(
    registration: &Registration,
    output: &Path,
    execution: &dyn ExecutionPort,
    store: &dyn InventoryPort,
) -> Result<CaseCount, Error> {
    if !crate::application::inventory_verification::service::verify(registration, output, execution, store)? {
        return Err(Error::Invalid(
            super::constants::REGISTERED_INDEPENDENT_CASE_CERTIFICATION_FA.into(),
        ));
    }
    Ok(CaseCount(
        registration.rules.iter().map(|rule| rule.cases.len()).sum(),
    ))
}

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
