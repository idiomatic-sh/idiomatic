//! The offline path reuses actual artifact loading and native receipt comparison.
use crate::domain::suite::types::inventory::Inventory;
use crate::outbound::inventory::types::{CapturedObservation, Registration};
use ryo_benchmark_contracts::domain::process::types::ReceiptAudit;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::collections::{BTreeMap, BTreeSet};

/// Validate every registered case, retaining valid pairs for resume while reporting every failure.
///
/// # Errors
/// Refuses malformed inventories or observations outside their declared identities.
pub fn validate(
    registration: &Registration,
    captured: &CapturedObservation,
) -> Result<ReceiptAudit, Error> {
    let _inventory = Inventory::new(&registration.defaults)?;
    let mut registered_rules = BTreeSet::new();
    let mut cases = BTreeMap::new();
    for rule in &registration.rules {
        if !registration.defaults.contains(&rule.rule_id) || !registered_rules.insert(&rule.rule_id)
        {
            return Err(Error::Invalid(
                crate::application::run::constants::UNKNOWN_OR_DUPLICATE_RECEIPT_REGISTRATION_RU
                    .into(),
            ));
        }
        for case in &rule.cases {
            let name = case[crate::application::run::constants::NAME]
                .as_str()
                .filter(|name| !name.is_empty())
                .ok_or_else(|| {
                    Error::Invalid(
                        crate::application::run::constants::REGISTERED_CASE_NAME_IS_MISSING.into(),
                    )
                })?;
            if case[crate::application::run::constants::RULE] != rule.rule_id
                || cases
                    .insert((rule.rule_id.clone(), name.to_owned()), case)
                    .is_some()
            {
                return Err(Error::Invalid(
                    crate::application::run::constants::UNKNOWN_OR_DUPLICATE_REGISTERED_CASE.into(),
                ));
            }
        }
    }
    if cases.is_empty() {
        return Err(Error::Invalid(
            crate::application::run::constants::RECEIPT_VALIDATION_REQUIRES_REGISTERED_CASES.into(),
        ));
    }
    let mut grouped = BTreeMap::<(String, String), Vec<serde_json::Value>>::new();
    for row in &captured.records {
        let key = (
            row[crate::application::run::constants::RULE]
                .as_str()
                .ok_or_else(|| {
                    Error::Invalid(crate::application::run::constants::MISSING_OBSERVED_RULE.into())
                })?
                .to_owned(),
            row[crate::application::run::constants::NAME]
                .as_str()
                .ok_or_else(|| {
                    Error::Invalid(crate::application::run::constants::MISSING_OBSERVED_CASE.into())
                })?
                .to_owned(),
        );
        if !cases.contains_key(&key)
            || !matches!(
                row[crate::application::run::constants::SIDE].as_str(),
                Some(
                    crate::application::run::constants::REFERENCE
                        | crate::application::run::constants::CANDIDATE
                )
            )
        {
            return Err(Error::Invalid(
                crate::application::run::constants::UNREGISTERED_RECEIPT_IDENTITY.into(),
            ));
        }
        grouped.entry(key).or_default().push(row.clone());
    }
    let mut audit = ReceiptAudit {
        case_count: cases.len(),
        ..ReceiptAudit::default()
    };
    for (key, case) in cases {
        let rows = grouped.remove(&key).unwrap_or_default();
        match super::inventory_receipts::validate_with_artifacts(
            std::slice::from_ref(case),
            &rows,
            &captured.artifacts,
        ) {
            Ok((raw, documented, contract_only)) => {
                let recomputed =
                    super::inventory_receipts::metadata_digest(case).map_err(Error::Invalid)?;
                if let Some(digest) = recomputed.as_str() {
                    audit.metadata_verified.push((
                        key.0.clone(),
                        key.1.clone(),
                        ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest(
                            digest.to_owned(),
                        ),
                    ));
                }
                if documented > 0
                    && case[crate::application::run::constants::REFERENCE_DISCREPANCY]
                        [crate::application::run::constants::BASIS]
                        == crate::application::run::constants::TOOL_CONTRACT_DIFFERENCE
                {
                    audit.tool_contract_differences.push(key.clone());
                }
                audit.verified.push(key);
                audit.raw_exact += raw;
                audit.documented_discrepancies += documented;
                audit.contract_only += contract_only;
            }
            Err(error) => audit.errors.push(format!("{}/{}: {error}", key.0, key.1)),
        }
    }
    Ok(audit)
}
