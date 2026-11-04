//! Actual shared receipt policy, including legacy compatibility and missing-mode forgery.
use super::*;
use serde_json::json;

fn pair(executable: bool) -> (Value, Vec<Value>) {
    let values = std::collections::BTreeMap::from([(
        ryo::linter::RelativePath("input.txt".into()),
        executable,
    )]);
    let digest = crate::domain::suite::input_metadata::digest(Some(&values))
        .unwrap()
        .unwrap()
        .0;
    let case = json!({"rule":"A001","name":"mode","source_sha256":"unchanged-bytes","files":["input.txt"],"expected":[],"reference_discrepancy":null,
        "file_observation":"posix-executable","file_metadata":{"input.txt":executable},"file_metadata_sha256":digest});
    let base = json!({"rule":"A001","name":"mode","complete":true,"error":null,"case_passed":true,"source_sha256":"unchanged-bytes","files":["input.txt"],"expected":[],"reference_expected":[],"diagnostics":[],"skipped":[],
        "observation_schema":"benchmark-case/v2","expectation_contract":"diagnostics","execution_metadata":{"availability":"complete"},"file_metadata_sha256":digest,
        "coverage":{"selected_rule_file_pairs":1,"evaluated_rule_file_pairs":1,"skipped_rule_file_pairs":0,"unknown_or_unread_rule_file_pairs":0}});
    let mut reference = base.clone();
    reference["side"] = json!("reference");
    let mut candidate = base;
    candidate["side"] = json!("candidate");
    (case, vec![reference, candidate])
}
#[test]
fn changed_mode_and_omitted_observations_cannot_reuse_byte_only_or_other_mode_proof() {
    for before in [false, true] {
        let (case, rows) = pair(before);
        assert_eq!(
            validate(std::slice::from_ref(&case), &rows).unwrap(),
            (1, 0, 0)
        );
        let (changed, _) = pair(!before);
        assert!(validate(&[changed], &rows).is_err());
        for key in ["file_metadata", "file_metadata_sha256"] {
            let mut missing = case.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(validate(&[missing], &rows).is_err());
        }
        let mut missing_rows = rows.clone();
        for row in &mut missing_rows {
            row.as_object_mut().unwrap().remove("file_metadata_sha256");
        }
        assert!(validate(&[case], &missing_rows).is_err());
    }
}
#[test]
fn genuinely_unspecified_legacy_cases_keep_their_existing_receipt_contract() {
    let (mut case, mut rows) = pair(false);
    for key in ["file_observation", "file_metadata", "file_metadata_sha256"] {
        case.as_object_mut().unwrap().remove(key);
    }
    for row in &mut rows {
        row.as_object_mut().unwrap().remove("file_metadata_sha256");
    }
    assert_eq!(validate(&[case], &rows).unwrap(), (1, 0, 0));
}

#[test]
fn offline_entry_emits_a_recomputed_witness_and_never_echoes_a_stale_declared_digest() {
    use crate::outbound::inventory::types::{CapturedObservation, Registration};
    let (case, records) = pair(false);
    let registration = |case: Value| -> Registration {
        serde_json::from_value(json!({"provider":"fixture","defaults":["A001"],"identity":{},"rules":[{"rule_id":"A001","command":null,"reason":null,"instance":null,"cases":[case]}]})).unwrap()
    };
    let captured = CapturedObservation {
        artifacts: Default::default(),
        records,
        sha256: Some("raw-receipt".into()),
    };
    let audit = crate::application::run::receipt_validation::validate(
        &registration(case.clone()),
        &captured,
    )
    .unwrap();
    assert!(audit.errors.is_empty());
    assert_eq!(audit.metadata_verified.len(), 1);
    assert_eq!(
        audit.metadata_verified[0].2.0,
        case["file_metadata_sha256"].as_str().unwrap()
    );
    let mut changed = case;
    changed["file_metadata"]["input.txt"] = json!(true); // Keep the old declaration/row digest to test recomputation.
    let audit =
        crate::application::run::receipt_validation::validate(&registration(changed), &captured)
            .unwrap();
    assert!(!audit.errors.is_empty());
    assert!(audit.verified.is_empty());
    assert!(audit.metadata_verified.is_empty());
}

#[test]
fn explicit_cohort_receipts_cannot_be_reused_for_other_or_omitted_selection() {
    let (mut case, mut rows) = pair(false);
    case["selected_rules"] = json!(["A001", "B002"]);
    for row in &mut rows {
        row["selected_rules"] = case["selected_rules"].clone();
    }
    assert_eq!(
        validate(std::slice::from_ref(&case), &rows).unwrap(),
        (1, 0, 0)
    );
    let mut different = case.clone();
    different["selected_rules"] = json!(["A001"]);
    assert!(validate(&[different], &rows).is_err());
    for side in 0..2 {
        let mut missing = rows.clone();
        missing[side]
            .as_object_mut()
            .unwrap()
            .remove("selected_rules");
        assert!(validate(std::slice::from_ref(&case), &missing).is_err());
    }
    case.as_object_mut().unwrap().remove("selected_rules");
    assert!(validate(&[case], &rows).is_err());
}

#[test]
fn explicit_observation_declarations_are_bound_to_both_durable_side_receipts() {
    let (mut case, mut rows) = pair(false);
    case["rule_observations"] = json!({"A001":[{"name":"matched","columns":[{"name":"file","kind":"file"}],"producers":[{"kind":"output","rule":"B002","output":"used λ\t\\source"}]}]});
    for row in &mut rows {
        row["rule_observations"] = case["rule_observations"].clone();
    }
    assert_eq!(
        validate(std::slice::from_ref(&case), &rows).unwrap(),
        (1, 0, 0)
    );
    for side in 0..2 {
        let mut changed = rows.clone();
        changed[side]["rule_observations"]["A001"][0]["producers"][0]["output"] = json!("other");
        assert!(validate(std::slice::from_ref(&case), &changed).is_err());
        let mut missing = rows.clone();
        missing[side]
            .as_object_mut()
            .unwrap()
            .remove("rule_observations");
        assert!(validate(std::slice::from_ref(&case), &missing).is_err());
    }
    case.as_object_mut().unwrap().remove("rule_observations");
    assert!(validate(&[case], &rows).is_err());
}
