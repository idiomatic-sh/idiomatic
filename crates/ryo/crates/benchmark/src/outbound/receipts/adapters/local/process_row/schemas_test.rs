//! What the process record accepts, writes back and refuses, one row of `CASES` for each input.
use super::ProcessRowSchema;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = &'static str;
/// Whether the record carries no raw artifacts, and whether it is written back as it was read.
type Value = (bool, bool);
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a record without raw artifacts is read as unattached"),
        word: Word::Succeeds,
        input: || r#"{"kind":"process-failure","exit":2,"signal":null,"actual_request_root":"/r","observer_no_deadline":true,"executable_sha256":null,"settings_sha256":null,"metadata_error":null,"template_sha256":null}"#,
        expect: Expect::Value((true, true)),
    },
    Case {
        name: CaseName("a record with raw artifacts is written back as it was read"),
        word: Word::Succeeds,
        input: || r#"{"kind":"process-failure","exit":2,"signal":9,"actual_request_root":"/r","observer_no_deadline":false,"executable_sha256":"e","settings_sha256":null,"metadata_error":"m","template_sha256":"t","artifacts":{"stdout":{"path":"out","sha256":"o"},"stderr":{"path":"err","sha256":"s"},"invocation":{"path":"call","sha256":"c"}}}"#,
        expect: Expect::Value((false, true)),
    },
    Case {
        name: CaseName("a field the record does not name is refused"),
        word: Word::Refuses,
        input: || r#"{"kind":"diagnostics","exit":2,"signal":null,"actual_request_root":"/r","observer_no_deadline":true,"executable_sha256":null,"settings_sha256":null,"metadata_error":null,"template_sha256":null,"surprise":1}"#,
        expect: Expect::Error(()),
    },
];

/// Decodes one record and answers whether it carries no raw artifacts, and whether writing it gives the text it was read from.
fn subject(text: Input) -> Result<Value, Failure> {
    let schema: ProcessRowSchema = serde_json::from_str(text).map_err(|_refused| ())?;
    let record = ryo_benchmark_contracts::domain::documents::types::observation_document::value::ProcessRecord::from(schema);
    let written = serde_json::to_string(&ProcessRowSchema::from(&record)).map_err(|_refused| ())?;
    Ok((
        record.artifacts
            == ryo_benchmark_contracts::domain::documents::types::observation_document::value::ProcessArtifactsRecord::Unattached,
        written == text,
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
