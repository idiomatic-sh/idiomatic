use super::*;
use serde_json::json;

fn fixture() -> (Value, Value, Value) {
    let case = json!({
        "rule":"R1", "name":"syntax", "files":["input"], "source_sha256":"source",
        "expected":[], "reference_discrepancy":null, "project_context_mode":"isolated",
        "expectation_contract":"diagnostics"
    });
    let mut candidate = json!({
        "rule":"R1", "name":"syntax", "side":"candidate", "files":["input"],
        "source_sha256":"source", "complete":true, "error":null, "case_passed":true,
        "expected":[], "reference_expected":[], "diagnostics":[],
        "skipped":[{"rule":"R1","path":"input","reason":"syntax-rejected"}],
        "observation_schema":"benchmark-case/v2", "expectation_contract":"diagnostics",
        "execution_metadata":{"availability":"complete"},
        "coverage":{"selected_rule_file_pairs":1,"evaluated_rule_file_pairs":0,
                    "skipped_rule_file_pairs":1,"unknown_or_unread_rule_file_pairs":0}
    });
    let mut reference = candidate.clone();
    reference["side"] = json!("reference");
    reference["execution_metadata"] =
        json!({"availability":"unavailable","reason":"findings-only CLI"});
    reference["coverage"] = Value::Null;
    reference["skipped"] = json!([]);
    candidate["context_manifest"] = Value::Null;
    (case, reference, candidate)
}

#[test]
fn unavailable_is_not_known_empty_and_contract_pass_is_not_raw_equality() {
    let (case, reference, candidate) = fixture();
    assert_eq!(
        compare(&reference, &candidate, ComparisonContract::Diagnostics),
        Ok(Comparison::Incomparable)
    );
    assert!(compare(&reference, &candidate, ComparisonContract::CompleteReport).is_err());
    assert_eq!(
        crate::application::inventory_receipts::service::validate(
            &[case],
            &[reference.clone(), candidate.clone()]
        ),
        Ok(
            crate::application::inventory_receipts::types::ReceiptCounts {
                raw_exact: crate::application::inventory_receipts::types::Tally(0),
                documented: crate::application::inventory_receipts::types::Tally(0),
                contract_only: crate::application::inventory_receipts::types::Tally(1)
            }
        )
    );
    assert_ne!(
        reference["skipped"], candidate["skipped"],
        "raw evidence must remain unchanged"
    );
}

#[test]
fn available_accounting_must_compare_real_skips() {
    let (_, mut reference, candidate) = fixture();
    reference["execution_metadata"] = json!({"availability":"complete"});
    reference["coverage"] = candidate["coverage"].clone();
    reference["coverage"]["evaluated_rule_file_pairs"] = json!(1);
    reference["coverage"]["skipped_rule_file_pairs"] = json!(0);
    assert!(compare(&reference, &candidate, ComparisonContract::Diagnostics).is_err());
    reference["skipped"] = candidate["skipped"].clone();
    reference["coverage"] = candidate["coverage"].clone();
    assert_eq!(
        compare(&reference, &candidate, ComparisonContract::CompleteReport),
        Ok(Comparison::Exact)
    );
}

#[test]
fn absent_forged_or_contradictory_capability_and_contract_are_refused() {
    let (case, reference, candidate) = fixture();
    for capability in [
        Value::Null,
        json!({"availability":"unknown"}),
        json!({"availability":"unavailable","reason":""}),
    ] {
        let mut wrong = reference.clone();
        wrong["execution_metadata"] = capability;
        assert!(compare(&wrong, &candidate, ComparisonContract::Diagnostics).is_err());
    }
    for value in [Value::Null, json!("unknown"), json!("complete-report")] {
        let mut wrong = reference.clone();
        wrong["expectation_contract"] = value;
        assert!(
            crate::application::inventory_receipts::service::validate(
                &[case.clone()],
                &[wrong, candidate.clone()]
            )
            .is_err()
        );
    }
    let mut wrong = reference.clone();
    wrong["skipped"] = candidate["skipped"].clone();
    assert!(compare(&wrong, &candidate, ComparisonContract::Diagnostics).is_err());
    let mut wrong = candidate;
    wrong["coverage"]["unknown_or_unread_rule_file_pairs"] = json!(1);
    assert!(compare(&reference, &wrong, ComparisonContract::Diagnostics).is_err());
}
