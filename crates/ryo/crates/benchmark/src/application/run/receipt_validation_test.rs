//! The offline entry recomputes the metadata witness and never echoes a stale declared digest.
use super::validate;
use crate::application::inventory_receipts::fixtures::with_file_metadata;
use crate::outbound::inventory::types::{
    CapturedObservation, RegisteredRule, Registration, RegistrationAuthority,
    case_document::CaseDocument, text::PathText,
};
use ryo_benchmark_contracts::domain::artifacts::types::FileExecutable;

fn registration(case: CaseDocument) -> Registration {
    Registration {
        authority: RegistrationAuthority::ReceiptOnly,
        provider: "fixture".into(),
        defaults: vec!["A001".into()],
        rules: vec![RegisteredRule {
            definition: None,
            modules: Default::default(),
            artifacts: Default::default(),
            rule_id: "A001".into(),
            command: None,
            reason: None,
            cases: vec![case],
            instance: None,
        }],
        identity: Default::default(),
    }
}

#[test]
fn offline_entry_emits_a_recomputed_witness_and_never_echoes_a_stale_declared_digest() {
    let (cases, records) = with_file_metadata(false);
    let case = cases[0].clone();
    let captured = CapturedObservation {
        artifacts: Default::default(),
        invocations: Default::default(),
        records,
        sha256: Some("raw-receipt".into()),
    };
    let audit = validate(&registration(case.clone()), &captured).unwrap();
    assert!(audit.errors.is_empty());
    assert_eq!(audit.metadata_verified.len(), 1);
    assert_eq!(
        Some(audit.metadata_verified[0].2.0.as_str()),
        case.file_metadata_sha256
            .as_ref()
            .map(|digest| digest.0.as_str())
    );
    let mut changed = case;
    // Keep the old declaration/row digest to test recomputation.
    changed.file_metadata = Some(
        [(PathText("input.txt".into()), FileExecutable::Set)]
            .into_iter()
            .collect(),
    );
    let audit = validate(&registration(changed), &captured).unwrap();
    assert!(!audit.errors.is_empty());
    assert!(audit.verified.is_empty());
    assert!(audit.metadata_verified.is_empty());
}
