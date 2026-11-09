//! Exact wire spellings and refusal of undeclared producer metadata.
use super::{ExecutionMetadataSchema, ReceiptSchema};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::ProjectContextReceipt;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::types::{ExecutionMetadata, UnavailableReason};
use std::collections::BTreeMap;
fn execution_metadata_roundtrips_exact_wire_without_a_serde_contract_on_neutral_data() {
    for (data, wire) in [
        (ExecutionMetadata::Unknown, r#"{"availability":"unknown"}"#),
        (
            ExecutionMetadata::Unavailable {
                reason: UnavailableReason("format limitation".into()),
            },
            r#"{"availability":"unavailable","reason":"format limitation"}"#,
        ),
        (
            ExecutionMetadata::Complete,
            r#"{"availability":"complete"}"#,
        ),
    ] {
        let encoded = serde_json::to_string(&ExecutionMetadataSchema::from(&data)).unwrap();
        assert_eq!(encoded, wire);
        let decoded: ExecutionMetadataSchema = serde_json::from_str(wire).unwrap();
        assert_eq!(ExecutionMetadata::from(decoded), data);
    }
}
fn missing_reason_unknown_capability_and_extra_wire_fields_are_refused() {
    for wire in [
        r#"{"availability":"unavailable"}"#,
        r#"{"availability":"assumed-complete"}"#,
        r#"{"availability":"complete","extra":true}"#,
        r#"{"availability":"unknown","extra":true}"#,
    ] {
        assert!(matches!(
            serde_json::from_str::<ExecutionMetadataSchema>(wire),
            Err(_refused_wire)
        ));
    }
}

fn receipts_encode_their_mode_and_unobserved_dependencies() {
    let receipt = ProjectContextReceipt {
        mode: ProjectContextMode::ListedDisk,
        logical_root: "pkg".into(),
        actual_root: "/snapshot/pkg".into(),
        manifest_hash: ArtifactDigest("context-hash".into()),
        dependencies: BTreeMap::from([("../__init__.py".into(), None)]),
    };
    let encoded = serde_json::to_value(ReceiptSchema::from(receipt)).expect("receipt JSON");
    assert_eq!(encoded["mode"], "listed-disk");
    assert_eq!(encoded["manifest_hash"], "context-hash");
    assert!(encoded["dependencies"]["../__init__.py"].is_null());
}

type Scenario = fn();
type Input = Scenario;
type Value = ();
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "execution_metadata_roundtrips_exact_wire_without_a_serde_contract_on_neutral_data",
        ),
        word: Word::Succeeds,
        input: || {
            execution_metadata_roundtrips_exact_wire_without_a_serde_contract_on_neutral_data
                as fn()
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("missing_reason_unknown_capability_and_extra_wire_fields_are_refused"),
        word: Word::Succeeds,
        input: || missing_reason_unknown_capability_and_extra_wire_fields_are_refused as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("receipts_encode_their_mode_and_unobserved_dependencies"),
        word: Word::Succeeds,
        input: || receipts_encode_their_mode_and_unobserved_dependencies as fn(),
        expect: Expect::Value(()),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
