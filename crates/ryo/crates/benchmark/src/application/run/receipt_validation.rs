//! The offline path reuses actual artifact loading and native receipt comparison.
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    suite::{process::types::ReceiptAudit, types::inventory::Inventory},
};
use crate::outbound::inventory::types::{CapturedObservation, Registration};
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
                "unknown or duplicate receipt registration rule".into(),
            ));
        }
        for case in &rule.cases {
            let name = case["name"]
                .as_str()
                .filter(|name| !name.is_empty())
                .ok_or_else(|| Error::Invalid("registered case name is missing".into()))?;
            if case["rule"] != rule.rule_id
                || cases
                    .insert((rule.rule_id.clone(), name.to_owned()), case)
                    .is_some()
            {
                return Err(Error::Invalid(
                    "unknown or duplicate registered case".into(),
                ));
            }
        }
    }
    if cases.is_empty() {
        return Err(Error::Invalid(
            "receipt validation requires registered cases".into(),
        ));
    }
    let mut grouped = BTreeMap::<(String, String), Vec<serde_json::Value>>::new();
    for row in &captured.records {
        let key = (
            row["rule"]
                .as_str()
                .ok_or_else(|| Error::Invalid("missing observed rule".into()))?
                .to_owned(),
            row["name"]
                .as_str()
                .ok_or_else(|| Error::Invalid("missing observed case".into()))?
                .to_owned(),
        );
        if !cases.contains_key(&key)
            || !matches!(row["side"].as_str(), Some("reference" | "candidate"))
        {
            return Err(Error::Invalid("unregistered receipt identity".into()));
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
                        crate::domain::suite::types::ArtifactDigest(digest.to_owned()),
                    ));
                }
                if documented > 0 && case["reference_discrepancy"]["basis"] == "tool-contract-difference" { audit.tool_contract_differences.push(key.clone()); }
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
