//! Enumerate every registered default, invoke the existing verifier, and retain every result.
use super::constants;
use crate::domain::suite::types::inventory::value::{
    FailureReason, Inventory, PassedRule, RuleEvidence, InventoryVerdict,
};
use crate::outbound::{
    execution::interface::ExecutionPort,
    inventory::{
        interface::InventoryPort,
        types::{CASE_RECEIPTS_ARGUMENT, Command, Registration, RegisteredRule, Row, Summary},
    },
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::requests::types::RuleLabel;
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

fn summary(registration: &Registration, inventory: &Inventory, state: &'static str) -> Summary {
    let rules = inventory
        .rules()
        .iter()
        .map(|result| {
            let instance = registration
                .rules
                .iter()
                .find(|entry| entry.rule_id == result.rule().0)
                .and_then(|entry| entry.instance.clone());
            match result.outcome() {
                RuleEvidence::Passed(evidence) => Row {
                    rule_id: result.rule().0.clone(),
                    status: constants::PASS,
                    reason: None,
                    instance,
                    case_results_sha256: Some(evidence.case_results_sha256.0.clone()),
                    case_count: evidence.cases.0,
                    raw_exact: evidence.raw_exact.0,
                    documented_discrepancies: evidence.documented.0,
                    contract_only: evidence.contract_only.0,
                },
                RuleEvidence::Failed(reason) => Row {
                    rule_id: result.rule().0.clone(),
                    status: constants::FAIL,
                    reason: Some(reason.0.clone()),
                    instance,
                    case_results_sha256: None,
                    case_count: 0,
                    raw_exact: 0,
                    documented_discrepancies: 0,
                    contract_only: 0,
                },
            }
        })
        .collect::<Vec<_>>();
    let pass = rules
        .iter()
        .filter(|row| row.status == constants::PASS)
        .count();
    Summary {
        schema: constants::BENCHMARK_INVENTORY_V1,
        provider: registration.provider.clone(),
        run_state: state,
        identity: registration.identity.clone(),
        counts: BTreeMap::from([
            (constants::PASS, pass),
            (constants::FAIL, rules.len() - pass),
        ]),
        rules,
    }
}
fn io_error(message: String) -> Error {
    Error::Invalid(message)
}

/// Run every pinned rule; one failed or missing implementation never removes another row.
///
/// # Errors
/// Refuses invalid registrations and propagates artifact persistence failures.
pub fn verify(
    registration: &Registration,
    output: &Path,
    execution: &dyn ExecutionPort,
    store: &dyn InventoryPort,
) -> Result<bool, Error> {
    require_authority(registration, store)?;
    let mut inventory = Inventory::new(
        &registration
            .defaults
            .iter()
            .cloned()
            .map(RuleLabel)
            .collect::<Vec<_>>(),
    )?;
    store.begin(output).map_err(io_error)?;
    store
        .publish(
            output,
            &summary(registration, &inventory, constants::RUNNING),
        )
        .map_err(io_error)?;
    let mut registered = BTreeSet::new();
    if registration.rules.iter().any(|rule| {
        !registration.defaults.contains(&rule.rule_id) || !registered.insert(&rule.rule_id)
    }) {
        inventory.fail_all(&FailureReason(
            constants::UNKNOWN_OR_DUPLICATE_REGISTERED_RULE.into(),
        ));
        store
            .publish(
                output,
                &summary(registration, &inventory, constants::FAILED),
            )
            .map_err(io_error)?;
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
            .find(|entry| entry.rule_id == rule.0);
        let outcome = (|| -> Result<(usize, usize, usize, usize, String), String> {
            let entry = entry.ok_or(constants::MISSING_IMPLEMENTATION_REGISTRATION)?;
            if let Some(reason) = &entry.reason {
                return Err(reason.clone());
            }
            let command = entry
                .command
                .as_ref()
                .ok_or(constants::MISSING_IMPLEMENTATION_INVOCATION)?;
            let Command::CaseVerification { context } = command;
            verify_context(entry, store)?;
            let definition = entry
                .definition
                .as_ref()
                .ok_or(constants::EXECUTABLE_RULE_HAS_NO_TESTED_DEFINITION)?;
            if entry.artifacts.get(&definition.path) != Some(&definition.sha256)
                || entry
                    .modules
                    .values()
                    .any(|pin| entry.artifacts.get(&pin.path) != Some(&pin.sha256))
            {
                return Err(constants::QUERY_MODULE_PINS_ARE_OUTSIDE_THE.into());
            }
            if entry.cases.is_empty() {
                return Err(constants::MISSING_INDEPENDENT_CASES.into());
            }
            if entry.cases.iter().any(|case| case.rule.0 != entry.rule_id) {
                return Err(constants::CASE_IDENTITY_BELONGS_TO_ANOTHER_REGISTERED.into());
            }
            let crate::outbound::inventory::types::RegistrationAuthority::Execution {
                reference,
                worker,
                ..
            } = &registration.authority
            else {
                return Err(constants::CASE_VERIFICATION_REQUIRES_EXECUTION_AUTHORI.into());
            };
            let mut destination = std::ffi::OsString::from(CASE_RECEIPTS_ARGUMENT);
            destination.push(directory.join(constants::CASES_JSONL).as_os_str());
            store.prepare(&directory)?;
            let invocation = Invocation {
                executable: worker.path.clone(),
                arguments: vec![
                    constants::VERIFY_CASES.into(),
                    reference.path.as_os_str().to_owned(),
                    context.as_os_str().to_owned(),
                    destination,
                ],
                environment: BTreeMap::new(),
                directory: context.clone(),
            };
            let observed = execution
                .run(invocation)
                .map_err(|error| error.to_string())?;
            let captured = store.capture(&directory, &observed.stdout.0, &observed.stderr.0)?;
            if observed.status.0 != 0 {
                return Err(format!(
                    "verifier exited {}: {}",
                    observed.status.0,
                    String::from_utf8_lossy(&observed.stderr.0)
                ));
            }
            let counts = if captured.artifacts.is_empty() {
                crate::domain::inventory_receipts::service::validate(
                    &entry.cases,
                    &captured.records,
                )?
            } else {
                crate::domain::inventory_receipts::service::validate_with_artifacts(
                    &entry.cases,
                    &captured.records,
                    &captured.artifacts,
                    &captured.invocations,
                )?
            };
            verify_context(entry, store)?;
            let digest = captured
                .sha256
                .ok_or(constants::NATIVE_RECEIPT_DIGEST_IS_MISSING)?;
            Ok((
                entry.cases.len(),
                counts.raw_exact.0,
                counts.documented.0,
                counts.contract_only.0,
                digest,
            ))
        })();
        match outcome {
            Ok((cases, raw, documented, contract_only, receipt)) => inventory.pass(
                &rule,
                PassedRule {
                    cases: CaseCount(cases),
                    raw_exact: CaseCount(raw),
                    documented: CaseCount(documented),
                    contract_only: CaseCount(contract_only),
                    case_results_sha256: ArtifactDigest(receipt),
                },
            ),
            Err(reason) => inventory.fail(&rule, FailureReason(reason)),
        }
        let report = summary(registration, &inventory, constants::RUNNING);
        store
            .result(&directory, &report.rules[index])
            .map_err(io_error)?;
        store.publish(output, &report).map_err(io_error)?;
    }
    require_authority(registration, store)?;
    store
        .publish(
            output,
            &summary(registration, &inventory, constants::COMPLETE),
        )
        .map_err(io_error)?;
    Ok(inventory.verdict() == InventoryVerdict::Passed)
}

/// A complete result cannot contradict its pinned canonical profile.
pub(crate) fn require_authority(
    registration: &Registration,
    store: &dyn InventoryPort,
) -> Result<(), Error> {
    use crate::outbound::inventory::types::RegistrationAuthority;
    if matches!(
        &registration.authority,
        RegistrationAuthority::Unavailable { .. }
    ) && registration.rules.iter().all(|r| r.command.is_none())
    {
        return Ok(());
    }
    let RegistrationAuthority::Execution {
        profile_sha256,
        reference,
        worker,
        ..
    } = &registration.authority
    else {
        return Err(Error::Invalid(
            constants::RECEIPT_ONLY_INVENTORY_CANNOT_AUTHORIZE_EXEC.into(),
        ));
    };
    let observed = store
        .canonical(&registration.authority)
        .map_err(Error::Invalid)?;
    let declared: std::collections::BTreeSet<_> = registration.defaults.iter().collect();
    let actual: std::collections::BTreeSet<_> = observed.defaults.iter().collect();
    if declared.is_empty()
        || declared.len() != registration.defaults.len()
        || actual.len() != observed.defaults.len()
        || declared != actual
        || observed.profile_sha256 != *profile_sha256
        || observed.reference_sha256 != reference.sha256
        || observed.worker_sha256 != worker.sha256
        || !observed.approved_reference.contains(&reference.sha256)
    {
        return Err(Error::Invalid(
            constants::CANONICAL_INVENTORY_PROFILE_EXECUTABLE_IDENT.into(),
        ));
    }
    for entry in &registration.rules {
        if entry.command.is_some()
            && (observed.deferred.contains(&entry.rule_id)
                || observed.pending.contains(&entry.rule_id))
        {
            return Err(Error::Invalid(format!(
                "canonical profile defers an executable registered rule: {}",
                entry.rule_id
            )));
        }
    }
    Ok(())
}

/// Validate the full context and its exhaustive module declarations, before and after native execution.
pub(crate) fn verify_context(entry: &RegisteredRule, store: &dyn InventoryPort) -> Result<(), String> {
    let Command::CaseVerification { context } = entry
        .command
        .as_ref()
        .ok_or(constants::MISSING_VERIFICATION_COMMAND)?;
    let actual = store.inspect_context(context, &entry.artifacts)?;
    if !actual.modules.keys().eq(entry.modules.keys()) {
        return Err(constants::REGISTERED_MODULE_INVENTORY_DIFFERS_FROM_PIN.into());
    }
    for (name, relative) in actual.modules {
        let pin = entry
            .modules
            .get(&name)
            .ok_or(constants::MISSING_DECLARED_MODULE_PIN)?;
        if pin.path != context.join(relative) || entry.artifacts.get(&pin.path) != Some(&pin.sha256)
        {
            return Err(format!(
                "registered module pin differs from pinned context profile: {name}"
            ));
        }
    }
    Ok(())
}
