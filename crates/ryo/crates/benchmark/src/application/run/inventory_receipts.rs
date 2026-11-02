//! Validate native case observations; expectation comparison stays in the existing verifier.
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
/// Require exact registered case identities and two actual complete, passing observations.
pub(super) fn validate(cases: &[Value], observations: &[Value]) -> Result<(usize, usize), String> {
    if cases.is_empty() || observations.len() != cases.len() * 2 {
        return Err("missing or extra native case receipts".into());
    }
    let mut raw_exact = 0;
    let mut documented = 0;
    for case in cases {
        let expected = normalized(&case["expected"])?;
        let certificate = &case["reference_discrepancy"];
        let reference_expected = if certificate.is_null() {
            expected.clone()
        } else {
            if normalized(&certificate["intended_expected"])? != expected {
                return Err("discrepancy intention differs from registered case".into());
            }
            normalized(&certificate["reference_expected"])?
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
            if row["complete"] != true
                || !row["error"].is_null()
                || row["case_passed"] != true
                || row["source_sha256"] != case["source_sha256"]
                || row["files"] != case["files"]
                || normalized(&row["expected"])? != expected
                || normalized(&row["reference_expected"])? != reference_expected
            {
                return Err("failed, incomplete, or stale native observation".into());
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
            if proof.is_null() || proof != &sides[1]["discrepancy_validation"] {
                return Err("missing independent discrepancy validation".into());
            }
            for (actual, expected) in [
                ("settings_sha256", "settings-sha256"),
                ("profile_sha256", "profile-sha256"),
                ("source_sha256", "source-sha256"),
                ("reference_output_sha256", "reference-output-sha256"),
                ("intended_output_sha256", "intended-output-sha256"),
            ] {
                if proof[actual] != certificate["identity"][expected] {
                    return Err("stale discrepancy evidence identity".into());
                }
            }
            if !certificate["approved_executable_sha256"]
                .as_array()
                .is_some_and(|pins| pins.contains(&proof["actual_executable_sha256"]))
            {
                return Err("discrepancy tool is not independently pinned".into());
            }
        }
        let exact = normalized(&sides[0]["diagnostics"])? == normalized(&sides[1]["diagnostics"])?
            && sides[0]["skipped"] == sides[1]["skipped"];
        raw_exact += usize::from(exact);
        documented += usize::from(!exact && !certificate.is_null());
    }
    Ok((raw_exact, documented))
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
            Ok((1, 0))
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
        assert_eq!(validate(&[case], &[reference, candidate]), Ok((0, 1)));
    }
}
