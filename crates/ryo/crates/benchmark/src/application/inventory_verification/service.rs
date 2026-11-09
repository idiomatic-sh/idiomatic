//! Enumerate every registered default, invoke the existing verifier, and retain every result.
use super::constants;
use crate::domain::suite::types::inventory::value::{
    FailureReason, Inventory, InventoryVerdict, PassedRule, RuleEvidence,
};
use crate::outbound::{
    execution::interface::ExecutionPort,
    inventory::{
        interface::InventoryPort,
        types::{
            Command, ReceiptDigest, ReceiptFile, RegisteredRule, Registration,
            RegistrationAuthority, Row, RowInstance, RowReason, RowStatus, RuleCommand,
            RuleDefinition, RuleInstance, RuleReason, RunState, Summary,
        },
    },
    receipts::{interface::ReceiptsPort, types::ReceiptPath},
    suite::{interface::SuitePort, types::ArtifactPath},
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::documents::types::text::value::{
    ReasonText, RuleName, Sha256Text,
};
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::receipts::constants::{
    CASE_RECEIPTS_ARGUMENT, CASES_JSONL, VERIFY_CASES,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::RuleLabel;
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// The row a rule's current result publishes.
fn row(registration: &Registration, rule: &RuleName, outcome: &RuleEvidence) -> Row {
    let instance = registration
        .rules
        .iter()
        .find(|entry| entry.rule_id == *rule)
        .map_or(RowInstance::Absent, |entry| match &entry.instance {
            RuleInstance::Declared(text) => RowInstance::Carried(text.clone()),
            RuleInstance::Undeclared => RowInstance::Absent,
        });
    match outcome {
        RuleEvidence::Passed(evidence) => Row {
            rule_id: rule.clone(),
            status: RowStatus::Pass,
            reason: RowReason::Passed,
            instance,
            case_results_sha256: ReceiptDigest::Recorded(Sha256Text(
                evidence.case_results_sha256.0.clone(),
            )),
            case_count: evidence.cases,
            raw_exact: evidence.raw_exact,
            documented_discrepancies: evidence.documented,
            contract_only: evidence.contract_only,
        },
        RuleEvidence::Failed(reason) => Row {
            rule_id: rule.clone(),
            status: RowStatus::Fail,
            reason: RowReason::Failed(ReasonText(reason.0.clone())),
            instance,
            case_results_sha256: ReceiptDigest::Unrecorded,
            case_count: CaseCount(0),
            raw_exact: CaseCount(0),
            documented_discrepancies: CaseCount(0),
            contract_only: CaseCount(0),
        },
    }
}

fn summary(registration: &Registration, inventory: &Inventory, state: RunState) -> Summary {
    let rules = inventory
        .rules()
        .iter()
        .map(|result| {
            row(
                registration,
                &RuleName(result.rule().0.clone()),
                result.outcome(),
            )
        })
        .collect::<Vec<_>>();
    let passed = rules
        .iter()
        .filter(|row| row.status == RowStatus::Pass)
        .count();
    Summary {
        provider: registration.provider.clone(),
        run_state: state,
        identity: registration.identity.clone(),
        passed: CaseCount(passed),
        failed: CaseCount(rules.len() - passed),
        rules,
    }
}

fn refused(error: impl std::fmt::Display) -> FailureReason {
    FailureReason(error.to_string())
}

/// Run every pinned rule; one failed or missing implementation never removes another row.
///
/// # Errors
/// Refuses invalid registrations and propagates artifact persistence failures.
pub fn verify(
    registration: &Registration,
    output: &Path,
    suite: &dyn SuitePort,
    receipts: &dyn ReceiptsPort,
    execution: &dyn ExecutionPort,
    store: &dyn InventoryPort,
) -> Result<InventoryVerdict, Error> {
    require_authority(registration, suite, store)?;
    let mut inventory = Inventory::new(
        &registration
            .defaults
            .iter()
            .map(|rule| RuleLabel(rule.0.clone()))
            .collect::<Vec<_>>(),
    )?;
    store.begin(output)?;
    store.publish(
        output,
        &summary(registration, &inventory, RunState::Running),
    )?;
    let mut registered = BTreeSet::new();
    if registration.rules.iter().any(|rule| {
        !registration.defaults.contains(&rule.rule_id) || !registered.insert(&rule.rule_id)
    }) {
        inventory.fail_all(&FailureReason(
            constants::UNKNOWN_OR_DUPLICATE_REGISTERED_RULE.into(),
        ));
        store.publish(output, &summary(registration, &inventory, RunState::Failed))?;
        return Err(Error::Invalid(
            constants::UNKNOWN_OR_DUPLICATE_REGISTERED_RULE.into(),
        ));
    }
    for index in 0..inventory.rules().len() {
        let rule = inventory.rules()[index].rule().clone();
        let directory = output.join(constants::PROFILES).join(&rule.0);
        let entry = registration
            .rules
            .iter()
            .find(|entry| entry.rule_id.0 == rule.0);
        let outcome = (|| -> Result<PassedRule, FailureReason> {
            let entry = entry.ok_or_else(|| {
                FailureReason(constants::MISSING_IMPLEMENTATION_REGISTRATION.into())
            })?;
            if let RuleReason::Stated(reason) = &entry.reason {
                return Err(FailureReason(reason.0.clone()));
            }
            let RuleCommand::Registered(Command::CaseVerification { context }) = &entry.command
            else {
                return Err(FailureReason(
                    constants::MISSING_IMPLEMENTATION_INVOCATION.into(),
                ));
            };
            verify_context(entry, suite, store)?;
            let RuleDefinition::Pinned(definition) = &entry.definition else {
                return Err(FailureReason(
                    constants::EXECUTABLE_RULE_HAS_NO_TESTED_DEFINITION.into(),
                ));
            };
            if entry.artifacts.get(&definition.path) != Some(&definition.sha256)
                || entry
                    .modules
                    .values()
                    .any(|pin| entry.artifacts.get(&pin.path) != Some(&pin.sha256))
            {
                return Err(FailureReason(
                    constants::QUERY_MODULE_PINS_ARE_OUTSIDE_THE.into(),
                ));
            }
            if entry.cases.is_empty() {
                return Err(FailureReason(constants::MISSING_INDEPENDENT_CASES.into()));
            }
            if entry.cases.iter().any(|case| case.rule != entry.rule_id) {
                return Err(FailureReason(
                    constants::CASE_IDENTITY_BELONGS_TO_ANOTHER_REGISTERED.into(),
                ));
            }
            let RegistrationAuthority::Execution {
                reference, worker, ..
            } = &registration.authority
            else {
                return Err(FailureReason(
                    constants::CASE_VERIFICATION_REQUIRES_EXECUTION_AUTHORI.into(),
                ));
            };
            let mut destination = std::ffi::OsString::from(CASE_RECEIPTS_ARGUMENT);
            destination.push(directory.join(CASES_JSONL).as_os_str());
            store.prepare(&directory).map_err(refused)?;
            let invocation = Invocation {
                executable: worker.path.clone(),
                arguments: vec![
                    VERIFY_CASES.into(),
                    reference.path.as_os_str().to_owned(),
                    context.as_os_str().to_owned(),
                    destination,
                ],
                environment: BTreeMap::new(),
                directory: context.clone(),
            };
            let observed = execution.run(invocation).map_err(refused)?;
            let receipt = store
                .capture(&directory, &observed.stdout, &observed.stderr)
                .map_err(refused)?;
            if observed.status.0 != 0 {
                return Err(FailureReason(format!(
                    "verifier exited {}: {}",
                    observed.status.0,
                    String::from_utf8_lossy(&observed.stderr.0)
                )));
            }
            let ReceiptFile::Present(path) = receipt else {
                return Err(FailureReason(
                    constants::NATIVE_RECEIPT_DIGEST_IS_MISSING.into(),
                ));
            };
            let records = receipts
                .case_observations(ReceiptPath(&path))
                .map_err(refused)?;
            let raw = store.raw_artifacts(&path, &records).map_err(refused)?;
            let counts = if raw.bytes.is_empty() {
                crate::domain::inventory_receipts::service::validate(&entry.cases, &records)
            } else {
                crate::domain::inventory_receipts::service::validate_with_artifacts(
                    &entry.cases,
                    &records,
                    &raw.bytes,
                    &raw.invocations,
                )
            }
            .map_err(refused)?;
            verify_context(entry, suite, store)?;
            let digest = suite.digest(ArtifactPath(&path)).map_err(refused)?;
            Ok(PassedRule {
                cases: CaseCount(entry.cases.len()),
                raw_exact: CaseCount(counts.raw_exact.0),
                documented: CaseCount(counts.documented.0),
                contract_only: CaseCount(counts.contract_only.0),
                case_results_sha256: ArtifactDigest(digest.0),
            })
        })();
        match outcome {
            Ok(evidence) => inventory.pass(&rule, evidence),
            Err(reason) => inventory.fail(&rule, reason),
        }
        let report = summary(registration, &inventory, RunState::Running);
        store.result(&directory, &report.rules[index])?;
        store.publish(output, &report)?;
    }
    require_authority(registration, suite, store)?;
    store.publish(
        output,
        &summary(registration, &inventory, RunState::Complete),
    )?;
    Ok(inventory.verdict())
}

/// Verify the registered cases once; cold, warm and edited measurements reuse the sealed result.
///
/// # Errors
/// Refuses a failed verification, and propagates every verification failure.
pub fn certify_registered(
    registration: &Registration,
    output: &Path,
    suite: &dyn SuitePort,
    receipts: &dyn ReceiptsPort,
    execution: &dyn ExecutionPort,
    store: &dyn InventoryPort,
) -> Result<CaseCount, Error> {
    if verify(registration, output, suite, receipts, execution, store)? != InventoryVerdict::Passed
    {
        return Err(Error::Invalid(
            constants::REGISTERED_INDEPENDENT_CASE_CERTIFICATION_FA.into(),
        ));
    }
    Ok(CaseCount(
        registration.rules.iter().map(|rule| rule.cases.len()).sum(),
    ))
}

/// A complete result cannot contradict its pinned canonical profile.
///
/// # Errors
/// Refuses a registration whose authority, defaults or executables differ from the canonical profile.
pub fn require_authority(
    registration: &Registration,
    suite: &dyn SuitePort,
    _store: &dyn InventoryPort,
) -> Result<(), Error> {
    if matches!(
        &registration.authority,
        RegistrationAuthority::Unavailable { .. }
    ) && registration
        .rules
        .iter()
        .all(|rule| rule.command == RuleCommand::Unregistered)
    {
        return Ok(());
    }
    let RegistrationAuthority::Execution {
        profile,
        profile_sha256,
        reference,
        worker,
    } = &registration.authority
    else {
        return Err(Error::Invalid(
            constants::RECEIPT_ONLY_INVENTORY_CANNOT_AUTHORIZE_EXEC.into(),
        ));
    };
    let tool = suite.profile(ArtifactPath(profile))?;
    let observed_profile = suite.digest(ArtifactPath(
        &profile.join(ryo_benchmark_contracts::domain::requests::constants::PROFILE_MANIFEST),
    ))?;
    let observed_reference = suite.digest(ArtifactPath(&reference.path))?;
    let observed_worker = suite.digest(ArtifactPath(&worker.path))?;
    let declared: BTreeSet<_> = registration.defaults.iter().collect();
    let observed: Vec<RuleName> = tool
        .defaults
        .iter()
        .map(|rule| RuleName(rule.to_string()))
        .collect();
    let actual: BTreeSet<_> = observed.iter().collect();
    if declared.is_empty()
        || declared.len() != registration.defaults.len()
        || actual.len() != observed.len()
        || declared != actual
        || observed_profile.0 != profile_sha256.0
        || observed_reference.0 != reference.sha256.0
        || observed_worker.0 != worker.sha256.0
        || !tool
            .sha256
            .iter()
            .any(|approved| approved.0 == reference.sha256.0)
    {
        return Err(Error::Invalid(
            constants::CANONICAL_INVENTORY_PROFILE_EXECUTABLE_IDENT.into(),
        ));
    }
    for entry in &registration.rules {
        let name = entry.rule_id.0.as_str();
        if entry.command != RuleCommand::Unregistered
            && (tool.deferred.keys().any(|rule| rule.to_string() == name)
                || tool.pending.keys().any(|rule| rule.to_string() == name))
        {
            return Err(Error::Invalid(format!(
                "canonical profile defers an executable registered rule: {name}"
            )));
        }
    }
    Ok(())
}

/// Validate the full context and its exhaustive module declarations, before and after native execution.
///
/// # Errors
/// Refuses a rule without a verification command, a context that differs from its pins, and a module inventory or pin that differs from the profile.
pub fn verify_context(
    entry: &RegisteredRule,
    suite: &dyn SuitePort,
    store: &dyn InventoryPort,
) -> Result<(), FailureReason> {
    let RuleCommand::Registered(Command::CaseVerification { context }) = &entry.command else {
        return Err(FailureReason(
            crate::domain::suite::constants::MISSING_VERIFICATION_COMMAND.into(),
        ));
    };
    if entry.artifacts.is_empty() {
        return Err(FailureReason(
            constants::REGISTERED_CASE_CONTEXT_IS_EMPTY.into(),
        ));
    }
    let listed = store.context_files(context).map_err(refused)?;
    if listed.files != entry.artifacts.keys().cloned().collect::<BTreeSet<_>>() {
        return Err(FailureReason(
            constants::REGISTERED_CONTEXT_NAMESPACE_DIFFERS_FROM_PI.into(),
        ));
    }
    for (path, expected) in &entry.artifacts {
        let actual = suite.digest(ArtifactPath(path)).map_err(refused)?;
        if actual.0 != expected.0 {
            return Err(FailureReason(format!(
                "registered case artifact changed: {}",
                path.display()
            )));
        }
    }
    let tool = suite.profile(ArtifactPath(context)).map_err(refused)?;
    if !tool
        .query_modules
        .keys()
        .map(|name| name.0.as_str())
        .eq(entry.modules.keys().map(|name| name.0.as_str()))
    {
        return Err(FailureReason(
            constants::REGISTERED_MODULE_INVENTORY_DIFFERS_FROM_PIN.into(),
        ));
    }
    for (name, relative) in &tool.query_modules {
        let pin = entry
            .modules
            .iter()
            .find(|(declared, _)| declared.0 == name.0)
            .map(|(_, pin)| pin)
            .ok_or_else(|| FailureReason(constants::MISSING_DECLARED_MODULE_PIN.into()))?;
        if pin.path != context.join(relative) || entry.artifacts.get(&pin.path) != Some(&pin.sha256)
        {
            return Err(FailureReason(format!(
                "registered module pin differs from pinned context profile: {}",
                name.0
            )));
        }
    }
    Ok(())
}
