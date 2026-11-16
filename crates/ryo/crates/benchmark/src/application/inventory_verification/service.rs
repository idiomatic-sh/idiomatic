//! Enumerate every registered default, invoke the existing verifier, and retain every result.
use super::authority::service::require_authority;
use super::constants;
use super::rule::service::verify_rule;
use super::types::RuleFiles;
use crate::domain::suite::service::covered;
use crate::domain::suite::types::inventory::value::{
    FailureReason, Inventory, InventoryVerdict, RuleEvidence,
};
use crate::outbound::evidence::types::{VerificationTimesFile, VerificationTimesFiles};
use crate::outbound::{
    execution::interface::ExecutionPort,
    inventory::{
        interface::InventoryPort,
        types::{
            ReceiptDigest, Registration, Row, RowInstance, RowReason, RowStatus, RuleInstance,
            RunState, Summary,
        },
    },
    receipts::interface::ReceiptsPort,
    scheduler::interface::SchedulerPort,
    suite::interface::SuitePort,
};
use ryo_benchmark_contracts::domain::documents::types::text::value::{
    ReasonText, RuleName, Sha256Text,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{
    RegisteredCoverage, RuleLabel,
    case_shard::value::{CaseShard, ShardCount, ShardIndex},
};
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use std::{collections::BTreeSet, path::Path};

/// The directory a rule's verification keeps its observations in: `<inventory>/profiles/<rule>`.
fn rule_directory(inventory: &Path, rule: &RuleLabel) -> std::path::PathBuf {
    inventory.join(constants::PROFILES).join(&rule.0)
}

/// The file each rule's verification writes its times to: `<inventory>/profiles/<rule>/verify-times.bmf`. The inventory
/// names it to each rule's verifier, and the bench reads the times of every rule it verified from it.
fn times_file(inventory: &Path, rule: &RuleLabel) -> VerificationTimesFile {
    VerificationTimesFile(rule_directory(inventory, rule).join(constants::VERIFY_TIMES_FILE))
}

/// The times file of each of `rules`, the rules the inventory at `inventory` verified.
#[must_use]
pub(crate) fn times_files(inventory: &Path, rules: &[ryo::linter::RuleId]) -> VerificationTimesFiles {
    VerificationTimesFiles(
        rules
            .iter()
            .map(|rule| (rule.clone(), times_file(inventory, &RuleLabel(rule.to_string()))))
            .collect(),
    )
}

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
        RuleEvidence::Deferred(capability) => Row {
            rule_id: rule.clone(),
            status: RowStatus::Deferred,
            reason: RowReason::Deferred(capability.clone()),
            instance,
            case_results_sha256: ReceiptDigest::Unrecorded,
            case_count: CaseCount(0),
            raw_exact: CaseCount(0),
            documented_discrepancies: CaseCount(0),
            contract_only: CaseCount(0),
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
    let count =
        |status: RowStatus| CaseCount(rules.iter().filter(|row| row.status == status).count());
    Summary {
        provider: registration.provider.clone(),
        run_state: state,
        identity: registration.identity.clone(),
        passed: count(RowStatus::Pass),
        failed: count(RowStatus::Fail),
        deferred: count(RowStatus::Deferred),
        rules,
    }
}

/// Run every pinned rule the coverage names; one failed or missing implementation never removes another row, and a
/// deferred default is recorded as deferred without running. The rules are dealt round robin to `shards` partitions the
/// scheduler runs at once; one writer records the results and publishes the summary, in the order the rules finish.
///
/// # Errors
/// Refuses a zero shard count, invalid registrations, and propagates artifact persistence failures.
#[expect(
    clippy::too_many_arguments,
    reason = "the verification forwards the five independently injected ports, the registration, its output, its coverage and the shard count"
)]
pub fn verify(
    registration: &Registration,
    output: &Path,
    suite: &dyn SuitePort,
    receipts: &dyn ReceiptsPort,
    execution: &dyn ExecutionPort,
    store: &dyn InventoryPort,
    coverage: &RegisteredCoverage,
    scheduler: &dyn SchedulerPort,
    shards: ShardCount,
) -> Result<InventoryVerdict, Error> {
    let partitions = (0..shards.0.max(1))
        .map(|index| CaseShard::new(ShardIndex(index), shards))
        .collect::<Result<Vec<_>, _>>()?;
    require_authority(registration, suite, store)?;
    let mut inventory = Inventory::new(
        &covered(coverage, &registration.defaults, |rule| rule)
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
    let rules = inventory
        .rules()
        .iter()
        .map(|result| result.rule().clone())
        .collect::<Vec<_>>();
    let (sender, finished) = std::sync::mpsc::channel();
    let mut written = Ok(());
    let mut tasks: Vec<Box<dyn FnOnce() + Send + '_>> = Vec::new();
    for shard in partitions {
        let sender = sender.clone();
        let rules = &rules;
        tasks.push(Box::new(move || {
            for (index, rule) in shard.select(rules.iter().enumerate()) {
                let directory = rule_directory(output, rule);
                let outcome = verify_rule(
                    registration,
                    registration
                        .rules
                        .iter()
                        .find(|entry| entry.rule_id.0 == rule.0),
                    RuleFiles {
                        directory: &directory,
                        times: &times_file(output, rule),
                    },
                    suite,
                    receipts,
                    execution,
                    store,
                );
                if sender.send((index, directory, outcome)).is_err() {
                    return;
                }
            }
        }));
    }
    drop(sender);
    {
        let (inventory, written, rules) = (&mut inventory, &mut written, &rules);
        // The one writer: it records each result as it finishes and publishes the summary, so no shard ever writes it.
        tasks.push(Box::new(move || {
            *written = finished
                .into_iter()
                .try_for_each(|(index, directory, outcome)| {
                    match outcome.unwrap_or_else(RuleEvidence::Failed) {
                        RuleEvidence::Passed(evidence) => inventory.pass(&rules[index], evidence),
                        RuleEvidence::Deferred(capability) => {
                            inventory.defer(&rules[index], &capability);
                        }
                        RuleEvidence::Failed(reason) => inventory.fail(&rules[index], reason),
                    }
                    let report = summary(registration, inventory, RunState::Running);
                    store.result(&directory, &report.rules[index])?;
                    store.publish(output, &report)
                });
        }));
    }
    scheduler.run_all(tasks);
    written?;
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
#[expect(
    clippy::too_many_arguments,
    reason = "the certification forwards the five independently injected ports, the registration, its output and its coverage"
)]
pub fn certify_registered(
    registration: &Registration,
    output: &Path,
    suite: &dyn SuitePort,
    receipts: &dyn ReceiptsPort,
    execution: &dyn ExecutionPort,
    store: &dyn InventoryPort,
    coverage: &RegisteredCoverage,
    scheduler: &dyn SchedulerPort,
) -> Result<CaseCount, Error> {
    if verify(
        registration,
        output,
        suite,
        receipts,
        execution,
        store,
        coverage,
        scheduler,
        ShardCount(1),
    )? != InventoryVerdict::Passed
    {
        return Err(Error::Invalid(
            constants::REGISTERED_INDEPENDENT_CASE_CERTIFICATION_FA.into(),
        ));
    }
    Ok(CaseCount(
        covered(coverage, &registration.rules, |rule| &rule.rule_id)
            .map(|rule| rule.cases.len())
            .sum(),
    ))
}
