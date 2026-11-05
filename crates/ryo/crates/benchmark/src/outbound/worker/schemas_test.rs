//! Exact wire spellings and refusal of undeclared producer metadata.
use super::ExecutionMetadataSchema;
use ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata;
#[test]
fn execution_metadata_roundtrips_exact_wire_without_a_serde_contract_on_neutral_data() {
    for (data, wire) in [
        (ExecutionMetadata::Unknown, r#"{"availability":"unknown"}"#),
        (
            ExecutionMetadata::Unavailable {
                reason: "format limitation".into(),
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
#[test]
fn missing_reason_unknown_capability_and_extra_wire_fields_are_refused() {
    for wire in [
        r#"{"availability":"unavailable"}"#,
        r#"{"availability":"assumed-complete"}"#,
        r#"{"availability":"complete","extra":true}"#,
        r#"{"availability":"unknown","extra":true}"#,
    ] {
        assert!(serde_json::from_str::<ExecutionMetadataSchema>(wire).is_err());
    }
}
