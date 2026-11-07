//! The offline path reuses actual artifact loading and native receipt comparison.
use super::constants;
use crate::domain::suite::types::inventory::value::Inventory;
use ryo_benchmark_contracts::domain::requests::types::RuleLabel;
use crate::outbound::inventory::types::{
    CapturedObservation, Registration,
    case_document::CaseDocument,
};
use ryo_benchmark_contracts::domain::documents::types::{observation_document::ObservationDocument, text::ContestantSide};
use ryo_benchmark_contracts::domain::receipts::types::{
    AuditFailure, CaseKey, MetadataVerifiedCase, ReceiptAudit,
};
use ryo_benchmark_contracts::domain::requests::types::RuleLabel;
use ryo_benchmark_contracts::domain::workloads::types::{
    benchmark_case_name::value::BenchmarkCaseName, case_count::value::CaseCount,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::DiscrepancyBasis;
use std::collections::{BTreeMap, BTreeSet};

/// Validate every registered case, retaining valid pairs for resume while reporting every failure.
///
/// # Errors
/// Refuses malformed inventories or observations outside their declared identities.
pub fn validate(
    registration: &Registration,
    captured: &CapturedObservation,
) -> Result<ReceiptAudit, Error> {
    let _inventory = Inventory::new(
        &registration
            .defaults
            .iter()
            .cloned()
            .map(RuleLabel)
            .collect::<Vec<_>>(),
    )?;
    let mut registered_rules = BTreeSet::new();
    let mut cases = BTreeMap::<(String, String), &CaseDocument>::new();
    for rule in &registration.rules {
        if !registration.defaults.contains(&rule.rule_id) || !registered_rules.insert(&rule.rule_id)
        {
            return Err(Error::Invalid(
                constants::UNKNOWN_OR_DUPLICATE_RECEIPT_REGISTRATION_RU.into(),
            ));
        }
        for case in &rule.cases {
            let name = Some(&case.name)
                .filter(|name| !name.0.is_empty())
                .ok_or_else(|| Error::Invalid(constants::REGISTERED_CASE_NAME_IS_MISSING.into()))?;
            if case.rule.0 != rule.rule_id
                || cases
                    .insert((rule.rule_id.clone(), name.0.clone()), case)
                    .is_some()
            {
                return Err(Error::Invalid(
                    constants::UNKNOWN_OR_DUPLICATE_REGISTERED_CASE.into(),
                ));
            }
        }
    }
    if cases.is_empty() {
        return Err(Error::Invalid(
            constants::RECEIPT_VALIDATION_REQUIRES_REGISTERED_CASES.into(),
        ));
    }
    let mut grouped = BTreeMap::<(String, String), Vec<ObservationDocument>>::new();
    for row in &captured.records {
        let key = (row.rule.0.clone(), row.name.0.clone());
        if !cases.contains_key(&key) || !matches!(row.side, ContestantSide::Reference | ContestantSide::Candidate) {
            return Err(Error::Invalid(
                constants::UNREGISTERED_RECEIPT_IDENTITY.into(),
            ));
        }
        grouped.entry(key).or_default().push(row.clone());
    }
    let mut audit = ReceiptAudit {
        case_count: CaseCount(cases.len()),
        ..ReceiptAudit::default()
    };
    for (key, case) in cases {
        let rows = grouped.remove(&key).unwrap_or_default();
        match crate::domain::inventory_receipts::service::validate_with_artifacts(
            std::slice::from_ref(case),
            &rows,
            &captured.artifacts,
            &captured.invocations,
        ) {
            Ok(counts) => {
                let recomputed = crate::domain::registered_inputs::service::metadata_digest(
                    &crate::domain::inventory_receipts::service::registered_inputs(case),
                )
                .map_err(|error| Error::Invalid(error.to_string()))?;
                let case_key = CaseKey {
                    rule: RuleLabel(key.0.clone()),
                    name: BenchmarkCaseName(key.1.clone()),
                };
                if let Some(digest) = recomputed {
                    audit.metadata_verified.push(MetadataVerifiedCase {
                        case: case_key.clone(),
                        digest,
                    });
                }
                if counts.documented.0 > 0
                    && case
                        .reference_discrepancy
                        .as_ref()
                        .is_some_and(|certificate| {
                            certificate.basis == DiscrepancyBasis::ToolContract
                        })
                {
                    audit.tool_contract_differences.push(case_key.clone());
                }
                audit.verified.push(case_key);
                audit.raw_exact.0 += counts.raw_exact.0;
                audit.documented_discrepancies.0 += counts.documented.0;
                audit.contract_only.0 += counts.contract_only.0;
            }
            Err(error) => audit
                .errors
                .push(AuditFailure(format!("{}/{}: {error}", key.0, key.1))),
        }
    }
    Ok(audit)
}
