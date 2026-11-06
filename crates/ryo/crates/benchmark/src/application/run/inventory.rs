//! Enumerate every registered default, invoke the existing verifier, and retain every result.
use crate::application::run::constants;
use crate::domain::suite::types::inventory::Inventory;
use crate::outbound::{
    execution::interface::ExecutionPort,
    inventory::{
        interface::InventoryPort,
        types::{CASE_RECEIPTS_ARGUMENT, Registration, Row, Summary},
    },
};
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

fn summary(registration: &Registration, inventory: &Inventory, state: &'static str) -> Summary {
    let rules = inventory
        .rules
        .iter()
        .map(|result| Row {
            rule_id: result.rule.clone(),
            status: if result.passed {
                constants::PASS
            } else {
                constants::FAIL
            },
            reason: result.reason.clone(),
            instance: registration
                .rules
                .iter()
                .find(|entry| entry.rule_id == result.rule)
                .and_then(|entry| entry.instance.clone()),
            case_results_sha256: result.case_results_sha256.clone(),
            case_count: result.cases,
            raw_exact: result.raw_exact,
            documented_discrepancies: result.documented,
            contract_only: result.contract_only,
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
    let mut inventory = Inventory::new(&registration.defaults)?;
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
        for row in &mut inventory.rules {
            row.reason = Some(constants::UNKNOWN_OR_DUPLICATE_REGISTERED_RULE.into());
        }
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
    for index in 0..inventory.rules.len() {
        let rule = &inventory.rules[index].rule;
        let directory = output.join(constants::PROFILES).join(rule);
        let entry = registration
            .rules
            .iter()
            .find(|entry| entry.rule_id == *rule);
        let outcome = (|| -> Result<(usize, usize, usize, usize, String), String> {
            let entry = entry.ok_or(constants::MISSING_IMPLEMENTATION_REGISTRATION)?;
            if let Some(reason) = &entry.reason {
                return Err(reason.clone());
            }
            let command = entry
                .command
                .as_ref()
                .ok_or(constants::MISSING_IMPLEMENTATION_INVOCATION)?;
            let crate::outbound::inventory::types::Command::CaseVerification { context } = command;
            super::registered_context::verify(entry, store)?;
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
            if entry
                .cases
                .iter()
                .any(|case| case[constants::RULE].as_str() != Some(entry.rule_id.as_str()))
            {
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
            let (raw, documented, contract_only) = if captured.artifacts.is_empty() {
                super::inventory_receipts::validate(&entry.cases, &captured.records)?
            } else {
                super::inventory_receipts::validate_with_artifacts(
                    &entry.cases,
                    &captured.records,
                    &captured.artifacts,
                )?
            };
            super::registered_context::verify(entry, store)?;
            let digest = captured
                .sha256
                .ok_or(constants::NATIVE_RECEIPT_DIGEST_IS_MISSING)?;
            Ok((entry.cases.len(), raw, documented, contract_only, digest))
        })();
        let row = &mut inventory.rules[index];
        match outcome {
            Ok((cases, raw, documented, contract_only, receipt)) => {
                row.case_results_sha256 = Some(receipt);
                row.passed = true;
                row.reason = None;
                row.cases = cases;
                row.raw_exact = raw;
                row.documented = documented;
                row.contract_only = contract_only;
            }
            Err(reason) => row.reason = Some(reason),
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
    Ok(inventory.all_passed())
}

#[cfg(test)]
#[path = "inventory_test.rs"]
mod tests;

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
