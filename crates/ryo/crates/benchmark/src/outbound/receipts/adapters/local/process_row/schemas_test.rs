//! What the process record accepts and refuses, one row of `CASES` for each input.
use super::ProcessRowSchema;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = &'static str;
type Value = bool;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a record without raw artifacts is read as unattached"),
        word: Word::Succeeds,
        input: || r#"{"kind":"process-failure","exit":2,"signal":null,"actual_request_root":"/r","observer_no_deadline":true,"executable_sha256":null,"settings_sha256":null,"metadata_error":null,"template_sha256":null}"#,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a field the record does not name is refused"),
        word: Word::Refuses,
        input: || r#"{"kind":"diagnostics","exit":2,"signal":null,"actual_request_root":"/r","observer_no_deadline":true,"executable_sha256":null,"settings_sha256":null,"metadata_error":null,"template_sha256":null,"surprise":1}"#,
        expect: Expect::Error(()),
    },
];

/// Decodes one record and answers whether it carries no raw artifacts.
fn subject(text: Input) -> Result<Value, Failure> {
    let schema: ProcessRowSchema = serde_json::from_str(text).map_err(|_refused| ())?;
    let record = ryo_benchmark_contracts::domain::documents::types::observation_document::value::ProcessRecord::from(schema);
    Ok(record.artifacts
        == ryo_benchmark_contracts::domain::documents::types::observation_document::value::ProcessArtifactsRecord::Unattached)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
