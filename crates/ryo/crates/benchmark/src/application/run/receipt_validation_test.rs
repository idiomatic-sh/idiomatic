//! The offline entry recomputes the metadata witness and never echoes a stale declared digest.
use super::validate;
use crate::application::inventory_receipts::fixtures::with_file_metadata;
use crate::outbound::inventory::types::{CapturedObservation, Registration};
use serde_json::{Value, json};

fn registration(case: Value) -> Registration {
    serde_json::from_value(json!({"authority":{"kind":"receipt-only"},"provider":"fixture","defaults":["A001"],"identity":{},"rules":[{"rule_id":"A001","definition":null,"modules":{},"artifacts":{},"command":null,"reason":null,"instance":null,"cases":[case]}]})).unwrap()
}

#[test]
fn offline_entry_emits_a_recomputed_witness_and_never_echoes_a_stale_declared_digest() {
    let (cases, records) = with_file_metadata(false);
    let case = cases[0].clone();
    let captured = CapturedObservation {
        artifacts: Default::default(),
        records,
        sha256: Some("raw-receipt".into()),
    };
    let audit = validate(&registration(case.clone()), &captured).unwrap();
    assert!(audit.errors.is_empty());
    assert_eq!(audit.metadata_verified.len(), 1);
    assert_eq!(
        audit.metadata_verified[0].2.0,
        case["file_metadata_sha256"].as_str().unwrap()
    );
    let mut changed = case;
    changed["file_metadata"]["input.txt"] = json!(true); // Keep the old declaration/row digest to test recomputation.
    let audit = validate(&registration(changed), &captured).unwrap();
    assert!(!audit.errors.is_empty());
    assert!(audit.verified.is_empty());
    assert!(audit.metadata_verified.is_empty());
}
