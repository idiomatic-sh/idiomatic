//! Enumerate every registered default, invoke the existing verifier, and retain every result.
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error, measurements::types::Invocation,
    suite::types::inventory::Inventory,
};
use crate::outbound::{
    execution::interface::ExecutionPort,
    inventory::{
        interface::InventoryPort,
        types::{Registration, Row, Summary},
    },
};
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
            status: if result.passed { "PASS" } else { "FAIL" },
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
        })
        .collect::<Vec<_>>();
    let pass = rules.iter().filter(|row| row.status == "PASS").count();
    Summary {
        schema: "benchmark-inventory/v1",
        provider: registration.provider.clone(),
        run_state: state,
        identity: registration.identity.clone(),
        counts: BTreeMap::from([("PASS", pass), ("FAIL", rules.len() - pass)]),
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
    let mut inventory = Inventory::new(&registration.defaults)?;
    store.begin(output).map_err(io_error)?;
    store
        .publish(output, &summary(registration, &inventory, "running"))
        .map_err(io_error)?;
    let mut registered = BTreeSet::new();
    if registration.rules.iter().any(|rule| {
        !registration.defaults.contains(&rule.rule_id) || !registered.insert(&rule.rule_id)
    }) {
        for row in &mut inventory.rules {
            row.reason = Some("unknown or duplicate registered rule".into());
        }
        store
            .publish(output, &summary(registration, &inventory, "failed"))
            .map_err(io_error)?;
        return Err(Error::Invalid(
            "unknown or duplicate registered rule".into(),
        ));
    }
    for index in 0..inventory.rules.len() {
        let rule = &inventory.rules[index].rule;
        let directory = output.join("profiles").join(rule);
        let entry = registration
            .rules
            .iter()
            .find(|entry| entry.rule_id == *rule);
        let outcome = (|| -> Result<(usize, usize, usize, String), String> {
            let entry = entry.ok_or("missing implementation registration")?;
            if let Some(reason) = &entry.reason {
                return Err(reason.clone());
            }
            let command = entry
                .command
                .as_ref()
                .ok_or("missing implementation invocation")?;
            if entry.cases.is_empty() {
                return Err("missing independent cases".into());
            }
            if entry
                .cases
                .iter()
                .any(|case| case["rule"].as_str() != Some(entry.rule_id.as_str()))
            {
                return Err("case identity belongs to another registered rule".into());
            }
            let mut environment = command
                .environment
                .iter()
                .map(|(key, value)| {
                    (
                        std::ffi::OsString::from(key.as_str()),
                        std::ffi::OsString::from(value.as_str()),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            let _previous = environment.insert(
                "RYO_VERIFY_CASE_RESULTS".into(),
                directory.join("cases.jsonl").into_os_string(),
            );
            store.prepare(&directory)?;
            let invocation = Invocation {
                executable: command.executable.clone(),
                arguments: command.arguments.iter().map(Into::into).collect(),
                environment,
                directory: command.directory.clone(),
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
            let (raw, documented) =
                super::inventory_receipts::validate(&entry.cases, &captured.records)?;
            let digest = captured.sha256.ok_or("native receipt digest is missing")?;
            Ok((entry.cases.len(), raw, documented, digest))
        })();
        let row = &mut inventory.rules[index];
        match outcome {
            Ok((cases, raw, documented, receipt)) => {
                row.case_results_sha256 = Some(receipt);
                row.passed = true;
                row.reason = None;
                row.cases = cases;
                row.raw_exact = raw;
                row.documented = documented;
            }
            Err(reason) => row.reason = Some(reason),
        }
        let report = summary(registration, &inventory, "running");
        store
            .result(&directory, &report.rules[index])
            .map_err(io_error)?;
        store.publish(output, &report).map_err(io_error)?;
    }
    store
        .publish(output, &summary(registration, &inventory, "complete"))
        .map_err(io_error)?;
    Ok(inventory.all_passed())
}

#[cfg(test)]
#[path = "inventory_tests.rs"]
mod tests;
