//! Validate native case observations; expectation comparison stays in the existing verifier.
use std::collections::HashSet;

use serde_json::Value;

fn normalized(value: &Value) -> Result<Vec<String>, String> {
    let source = value.as_array().ok_or("diagnostics must be an array")?;
    let mut rows = Vec::with_capacity(source.len());
    for value in source {
        let rule = value["rule"].as_str().ok_or("diagnostic rule is missing")?;
        let file = value["file"].as_str().ok_or("diagnostic file is missing")?;
        let start = value["start"]
            .as_u64()
            .ok_or("diagnostic start is invalid")?;
        let end = value["end"].as_u64().ok_or("diagnostic end is invalid")?;
        if rule.is_empty() || file.is_empty() || start > end {
            return Err("diagnostic span or identity is invalid".into());
        }
        rows.push(value.to_string());
    }
    rows.sort();
    Ok(rows)
}
/// Decode only declared metadata and recompute its pure framed identity.
pub(super) fn metadata_digest(case: &Value) -> Result<Value, String> {
    let files = case["files"]
        .as_array()
        .ok_or("registered files are missing")?;
    let required = !case["file_observation"].is_null();
    if required && case["file_observation"] != "posix-executable" {
        return Err("unsupported registered file observation".into());
    }
    let metadata = if case["file_metadata"].is_null() {
        None
    } else {
        let values = case["file_metadata"]
            .as_object()
            .ok_or("file metadata must be an object")?;
        let mut parsed = std::collections::BTreeMap::new();
        for (path, value) in values {
            crate::domain::suite::service::relative_path(std::path::Path::new(path))
                .map_err(|error| error.to_string())?;
            if !files.iter().any(|file| file.as_str() == Some(path)) {
                return Err("metadata names an undeclared source".into());
            }
            parsed.insert(
                ryo::linter::RelativePath(path.clone().into_boxed_str()),
                value
                    .as_bool()
                    .ok_or("executable observation must be boolean")?,
            );
        }
        Some(parsed)
    };
    if required
        && metadata
            .as_ref()
            .is_none_or(|values| values.len() != files.len())
    {
        return Err("required input observations are missing".into());
    }
    Ok(serde_json::json!(
        crate::domain::suite::input_metadata::digest(metadata.as_ref())
            .map_err(|error| error.to_string())?
            .map(|value| value.0)
    ))
}

/// Recompute the project input identity from the registered explicit roots and entry inventory.
fn project_inputs_digest(case: &Value) -> Result<Value, String> {
    let roots = if case["project_search_roots"].is_null() {
        None
    } else {
        Some(
            case["project_search_roots"]
                .as_array()
                .ok_or("project search roots must be an ordered array")?
                .iter()
                .map(|value| {
                    let root = value.as_str().ok_or("project search root is not text")?;
                    if !root.is_empty() {
                        crate::domain::suite::service::relative_path(root)
                            .map_err(|error| error.to_string())?;
                    }
                    Ok(ryo::linter::RelativePath(root.to_owned().into_boxed_str()))
                })
                .collect::<Result<Vec<_>, String>>()?,
        )
    };
    let inventory = if case["given_project_inventory"].is_null() {
        ryo::linter::GivenProjectInventory::default()
    } else {
        let wire: crate::outbound::context::adapters::local::schemas::ProjectInventorySchema =
            serde_json::from_value(case["given_project_inventory"].clone())
                .map_err(|error| error.to_string())?;
        wire.into()
    };
    for entry in &inventory.entries {
        crate::domain::suite::service::relative_path(entry.path.0.as_ref())
            .map_err(|error| error.to_string())?;
    }
    Ok(serde_json::json!(
        crate::domain::suite::project_inputs::digest(roots.as_deref(), &inventory)
            .map_err(|error| error.to_string())?
            .map(|digest| digest.0)
    ))
}

/// Require exact registered case identities and two actual complete, passing observations.
pub(super) fn validate(
    cases: &[Value],
    observations: &[Value],
) -> Result<(usize, usize, usize), String> {
    validate_with_artifacts(cases, observations, &std::collections::BTreeMap::new())
}
/// The same validation with exact raw process bytes loaded by the artifact boundary.
pub(super) fn validate_with_artifacts(
    cases: &[Value],
    observations: &[Value],
    artifacts: &std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
) -> Result<(usize, usize, usize), String> {
    if cases.is_empty() || observations.len() != cases.len() * 2 {
        return Err("missing or extra native case receipts".into());
    }
    let mut registered = HashSet::with_capacity(cases.len());
    for case in cases {
        let rule = case["rule"]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or("registered rule identity is missing")?;
        let name = case["name"]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or("registered case identity is missing")?;
        if !registered.insert((rule, name)) {
            return Err("duplicate registered case identity".into());
        }
    }
    let mut observed = HashSet::with_capacity(observations.len());
    for row in observations {
        let rule = row["rule"]
            .as_str()
            .ok_or("native rule identity is missing")?;
        let name = row["name"]
            .as_str()
            .ok_or("native case identity is missing")?;
        let side = row["side"]
            .as_str()
            .ok_or("native side identity is missing")?;
        if !registered.contains(&(rule, name)) || !matches!(side, "reference" | "candidate") {
            return Err("unregistered native case observation".into());
        }
        if !observed.insert((rule, name, side)) {
            return Err("duplicate native case observation".into());
        }
    }
    let mut raw_exact = 0;
    let mut documented = 0;
    let mut contract_only = 0;
    for case in cases {
        let metadata = metadata_digest(case)?;
        let project_inputs = project_inputs_digest(case)?;
        if case["file_metadata_sha256"] != metadata {
            return Err("registered file metadata digest differs".into());
        }
        if case["project_inputs_sha256"] != project_inputs {
            return Err("registered project inputs digest differs".into());
        }
        let declared = if case["expectation_contract"].is_null() {
            crate::domain::diagnostics::types::ComparisonContract::Diagnostics
        } else {
            super::inventory_execution::contract(&case["expectation_contract"])?
        };
        let expected = normalized(&case["expected"])?;
        let certificate = &case["reference_discrepancy"];
        let basis = if certificate.is_null() {
            "none"
        } else {
            certificate["basis"].as_str().unwrap_or("language-defect")
        };
        if !matches!(
            basis,
            "none" | "language-defect" | "tool-contract-difference"
        ) {
            return Err("unknown discrepancy authority".into());
        }
        let process = !certificate["process"].is_null();
        if process && declared != crate::domain::diagnostics::types::ComparisonContract::Diagnostics
        {
            return Err("process expectation cannot satisfy complete-report accounting".into());
        }
        let reference_expected = if certificate.is_null() {
            expected.clone()
        } else {
            if normalized(&certificate["intended_expected"])? != expected {
                return Err("discrepancy intention differs from registered case".into());
            }
            if process {
                Vec::new()
            } else {
                normalized(&certificate["reference_expected"])?
            }
        };
        let mut sides = Vec::new();
        for side in ["reference", "candidate"] {
            let found = observations
                .iter()
                .filter(|row| {
                    row["rule"] == case["rule"]
                        && row["name"] == case["name"]
                        && row["side"] == side
                })
                .collect::<Vec<_>>();
            if found.len() != 1 {
                return Err("missing or duplicate native side receipt".into());
            }
            let row = found[0];
            if row["rule_context"] != case["rule_context"] {
                return Err("native configured rule context differs from registration".into());
            }
            if row["rule_observations"] != case["rule_observations"] {
                return Err("native observation declarations differ from registration".into());
            }
            if row["selected_rules"] != case["selected_rules"] {
                return Err("native selected-rule cohort differs from registration".into());
            }
            if row["observation_schema"] != "benchmark-case/v2"
                || super::inventory_execution::contract(&row["expectation_contract"])? != declared
            {
                return Err("native expectation contract differs from registration".into());
            }
            if row["complete"] != true
                || !row["error"].is_null()
                || row["case_passed"] != true
                || row["source_sha256"] != case["source_sha256"]
                || row["file_metadata_sha256"] != metadata
                || row["project_inputs_sha256"] != project_inputs
                || row["files"] != case["files"]
                || normalized(&row["expected"])? != expected
                || if process {
                    !row["reference_expected"].is_null()
                } else {
                    normalized(&row["reference_expected"])? != reference_expected
                }
            {
                return Err("failed, incomplete, or stale native observation".into());
            }
            if row["documented_reference_discrepancy"]
                .as_bool()
                .unwrap_or(false)
                != (basis == "language-defect")
                || row["documented_tool_contract_difference"]
                    .as_bool()
                    .unwrap_or(false)
                    != (basis == "tool-contract-difference")
            {
                return Err("native discrepancy authority differs from registration".into());
            }
            if process && side == "reference" {
                super::process_receipts::validate(case, row, artifacts)?;
            } else {
                if !row["process_observation"].is_null() {
                    return Err("diagnostic report unexpectedly contains a process failure".into());
                }
                let actual = normalized(&row["diagnostics"])?;
                let required = if side == "reference" {
                    &reference_expected
                } else {
                    &expected
                };
                if actual.as_slice() != required.as_slice() || !row["skipped"].is_array() {
                    return Err("native diagnostics differ from registered expectations".into());
                }
            }
            if side == "candidate" {
                let coverage = &row["coverage"];
                let selected = coverage["selected_rule_file_pairs"].as_u64().unwrap_or(0);
                let evaluated = coverage["evaluated_rule_file_pairs"]
                    .as_u64()
                    .unwrap_or(u64::MAX);
                let skipped = coverage["skipped_rule_file_pairs"]
                    .as_u64()
                    .unwrap_or(u64::MAX);
                if row["observation_schema"] != "benchmark-case/v2"
                    || row["expectation_contract"] != "diagnostics"
                    || row["execution_metadata"]["availability"] != "complete"
                    || selected == 0
                    || evaluated.checked_add(skipped) != Some(selected)
                    || coverage["unknown_or_unread_rule_file_pairs"] != 0
                {
                    return Err("candidate execution is not completely accounted".into());
                }
            }
            if case["project_context_mode"] == "isolated" {
                if !row["context_manifest"].is_null() || !row["project_context"].is_null() {
                    return Err("unexpected project-context receipt".into());
                }
            } else {
                for key in ["mode", "logical_root", "manifest_hash"] {
                    if row["context_manifest"][key] != case["context_identity"][key] {
                        return Err("stale project-context receipt".into());
                    }
                }
            }
            sides.push(row);
        }
        if !certificate.is_null() {
            let proof = &sides[0]["discrepancy_validation"];
            if proof["file_metadata_sha256"] != metadata
                || certificate["identity"]["file-metadata-sha256"] != metadata
            {
                return Err("discrepancy file metadata identity differs".into());
            }
            if proof.is_null() || proof != &sides[1]["discrepancy_validation"] {
                return Err("missing independent discrepancy validation".into());
            }
            if proof["discrepancy_basis"]
                .as_str()
                .unwrap_or("language-defect")
                != basis
            {
                return Err("discrepancy authority was not verified".into());
            }
            for (actual, expected) in [
                ("settings_sha256", "settings-sha256"),
                ("profile_sha256", "profile-sha256"),
                ("source_sha256", "source-sha256"),
                ("reference_output_sha256", "reference-output-sha256"),
                ("intended_output_sha256", "intended-output-sha256"),
            ] {
                if !proof[actual]
                    .as_str()
                    .is_some_and(|value| !value.is_empty())
                    || !certificate["identity"][expected]
                        .as_str()
                        .is_some_and(|value| !value.is_empty())
                    || proof[actual] != certificate["identity"][expected]
                {
                    return Err("stale discrepancy evidence identity".into());
                }
            }
            if !proof["actual_executable_sha256"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
            {
                return Err("discrepancy tool identity is missing".into());
            }
            if !certificate["approved_executable_sha256"]
                .as_array()
                .is_some_and(|pins| pins.contains(&proof["actual_executable_sha256"]))
            {
                return Err("discrepancy tool is not independently pinned".into());
            }
        }
        super::inventory_execution::compare(sides[0], sides[1], declared)?;
        let exact = !process
            && normalized(&sides[0]["diagnostics"])? == normalized(&sides[1]["diagnostics"])?
            && sides[0]["skipped"] == sides[1]["skipped"];
        raw_exact += usize::from(exact);
        documented += usize::from(!exact && !certificate.is_null());
        contract_only += usize::from(!exact && certificate.is_null());
    }
    Ok((raw_exact, documented, contract_only))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn forged_pass_flags_cannot_hide_wrong_or_malformed_diagnostics() {
        let case = json!({
            "rule": "A001", "name": "case", "source_sha256": "source-pin",
            "files": ["input.txt"], "expected": [],
            "project_context_mode": "isolated", "reference_discrepancy": null
        });
        let base = json!({
            "rule": "A001", "name": "case", "complete": true, "error": null,
            "case_passed": true, "source_sha256": "source-pin", "files": ["input.txt"],
            "expected": [], "reference_expected": [], "diagnostics": [], "skipped": [],
            "context_manifest": null, "project_context": null,
            "observation_schema": "benchmark-case/v2", "expectation_contract": "diagnostics",
            "execution_metadata": {"availability": "complete"},
            "coverage": {"selected_rule_file_pairs": 1, "evaluated_rule_file_pairs": 1,
                         "skipped_rule_file_pairs": 0, "unknown_or_unread_rule_file_pairs": 0}
        });
        let mut reference = base.clone();
        reference["side"] = json!("reference");
        let mut candidate = base;
        candidate["side"] = json!("candidate");
        assert_eq!(
            validate(&[case.clone()], &[reference.clone(), candidate.clone()]),
            Ok((1, 0, 0))
        );

        candidate["diagnostics"] =
            json!([{"rule": "A001", "file": "input.txt", "start": 0, "end": 1}]);
        assert!(
            validate(&[case.clone()], &[reference.clone(), candidate.clone()])
                .unwrap_err()
                .contains("native diagnostics differ")
        );

        candidate["diagnostics"] = json!({"not": "an array"});
        assert!(
            validate(&[case], &[reference, candidate])
                .unwrap_err()
                .contains("diagnostics must be an array")
        );
    }

    #[test]
    fn registered_and_observed_case_identities_must_be_bijective() {
        let case = json!({
            "rule": "A001", "name": "first", "source_sha256": "source-pin",
            "files": ["input.txt"], "expected": [],
            "project_context_mode": "isolated", "reference_discrepancy": null
        });
        let base = json!({
            "rule": "A001", "name": "first", "complete": true, "error": null,
            "case_passed": true, "source_sha256": "source-pin", "files": ["input.txt"],
            "expected": [], "reference_expected": [], "diagnostics": [], "skipped": [],
            "context_manifest": null, "project_context": null,
            "observation_schema": "benchmark-case/v2", "expectation_contract": "diagnostics",
            "execution_metadata": {"availability": "complete"},
            "coverage": {"selected_rule_file_pairs": 1, "evaluated_rule_file_pairs": 1,
                         "skipped_rule_file_pairs": 0, "unknown_or_unread_rule_file_pairs": 0}
        });
        let mut reference = base.clone();
        reference["side"] = json!("reference");
        let mut candidate = base;
        candidate["side"] = json!("candidate");
        let mut unrelated_reference = reference.clone();
        unrelated_reference["name"] = json!("unrelated");
        let mut unrelated_candidate = candidate.clone();
        unrelated_candidate["name"] = json!("unrelated");
        let rows = [
            reference.clone(),
            candidate.clone(),
            unrelated_reference,
            unrelated_candidate,
        ];

        assert!(
            validate(&[case.clone(), case.clone()], &rows)
                .unwrap_err()
                .contains("duplicate registered case identity")
        );
        let mut second = case;
        second["name"] = json!("second");
        assert!(
            validate(
                &[
                    second.clone(),
                    json!({
                        "rule": "A001", "name": "first", "source_sha256": "source-pin",
                        "files": ["input.txt"], "expected": [],
                        "project_context_mode": "isolated", "reference_discrepancy": null
                    })
                ],
                &rows
            )
            .unwrap_err()
            .contains("unregistered native case observation")
        );

        let duplicate_rows = [reference.clone(), candidate.clone(), reference, candidate];
        assert!(
            validate(
                &[
                    second,
                    json!({
                        "rule": "A001", "name": "first", "source_sha256": "source-pin",
                        "files": ["input.txt"], "expected": [],
                        "project_context_mode": "isolated", "reference_discrepancy": null
                    })
                ],
                &duplicate_rows
            )
            .unwrap_err()
            .contains("duplicate native case observation")
        );
    }

    #[test]
    fn independently_bound_reference_difference_can_pass() {
        let finding = json!({"rule": "A001", "file": "input.txt", "start": 0, "end": 1});
        let case = json!({
            "rule": "A001", "name": "case", "source_sha256": "source-pin",
            "files": ["input.txt"], "expected": [], "project_context_mode": "isolated",
            "reference_discrepancy": {
                "intended_expected": [], "reference_expected": [finding],
                "approved_executable_sha256": ["tool-pin"],
                "identity": {
                    "settings-sha256": "settings-pin", "profile-sha256": "profile-pin",
                    "source-sha256": "source-pin", "reference-output-sha256": "reference-pin",
                    "intended-output-sha256": "intended-pin"
                }
            }
        });
        let proof = json!({
            "actual_executable_sha256": "tool-pin", "settings_sha256": "settings-pin",
            "profile_sha256": "profile-pin", "source_sha256": "source-pin",
            "reference_output_sha256": "reference-pin", "intended_output_sha256": "intended-pin"
        });
        let base = json!({
            "rule": "A001", "name": "case", "complete": true, "error": null,
            "case_passed": true, "source_sha256": "source-pin", "files": ["input.txt"],
            "expected": [], "reference_expected": [finding], "diagnostics": [], "skipped": [],
            "discrepancy_validation": proof,
            "context_manifest": null, "project_context": null,
            "observation_schema": "benchmark-case/v2", "expectation_contract": "diagnostics",
            "execution_metadata": {"availability": "complete"},
            "coverage": {"selected_rule_file_pairs": 1, "evaluated_rule_file_pairs": 1,
                         "skipped_rule_file_pairs": 0, "unknown_or_unread_rule_file_pairs": 0}
        });
        let mut reference = base.clone();
        reference["side"] = json!("reference");
        reference["diagnostics"] = json!([finding]);
        let mut candidate = base;
        candidate["side"] = json!("candidate");
        assert_eq!(
            validate(&[case.clone()], &[reference.clone(), candidate.clone()]),
            Ok((0, 1, 0))
        );

        let mut unbound = case;
        unbound["reference_discrepancy"]["identity"]["settings-sha256"] = Value::Null;
        reference["discrepancy_validation"]["settings_sha256"] = Value::Null;
        candidate["discrepancy_validation"]["settings_sha256"] = Value::Null;
        assert!(
            validate(&[unbound], &[reference, candidate])
                .unwrap_err()
                .contains("stale discrepancy evidence identity")
        );
    }
}

#[cfg(test)]
mod input_metadata_test;
