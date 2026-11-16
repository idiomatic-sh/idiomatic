//! One rule's verification: its registration and context are checked, the sealed worker verifies its cases, and the
//! receipts it leaves are validated against the registered cases.
use super::super::authority::service::{refused, verify_context};
use super::super::constants;
use crate::domain::suite::types::inventory::value::{FailureReason, PassedRule, RuleEvidence};
use crate::outbound::{
    execution::interface::ExecutionPort,
    inventory::{
        interface::InventoryPort,
        types::{
            Command, ReceiptFile, RegisteredRule, Registration, RegistrationAuthority, RuleCommand,
            RuleDefinition, RuleReason,
        },
    },
    receipts::{interface::ReceiptsPort, types::ReceiptPath},
    suite::{interface::SuitePort, types::ArtifactPath},
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::receipts::constants::{
    CASE_RECEIPTS_ARGUMENT, CASES_JSONL, VERIFY_CASES,
};
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use std::{collections::BTreeMap, path::Path};

/// Verify one rule of the registration, keeping its observations in `directory`; a refusal is the rule's failure, never
/// the inventory's, and a deferred default answers its capability without running.
pub(in crate::application::inventory_verification) fn verify_rule(
    registration: &Registration,
    entry: Option<&RegisteredRule>,
    directory: &Path,
    suite: &dyn SuitePort,
    receipts: &dyn ReceiptsPort,
    execution: &dyn ExecutionPort,
    store: &dyn InventoryPort,
) -> Result<RuleEvidence, FailureReason> {
    let entry = entry
        .ok_or_else(|| FailureReason(constants::MISSING_IMPLEMENTATION_REGISTRATION.into()))?;
    if let RuleCommand::Deferred(capability) = &entry.command {
        // `require_authority` refused every deferral the verifier cannot accept before any rule runs.
        return Ok(RuleEvidence::Deferred(capability.clone()));
    }
    if let RuleReason::Stated(reason) = &entry.reason {
        return Err(FailureReason(reason.0.clone()));
    }
    let RuleCommand::Registered(Command::CaseVerification { context }) = &entry.command else {
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
    Ok(RuleEvidence::Passed(PassedRule {
        cases: CaseCount(entry.cases.len()),
        raw_exact: CaseCount(counts.raw_exact.0),
        documented: CaseCount(counts.documented.0),
        contract_only: CaseCount(counts.contract_only.0),
        case_results_sha256: ArtifactDigest(digest.0),
    }))
}
