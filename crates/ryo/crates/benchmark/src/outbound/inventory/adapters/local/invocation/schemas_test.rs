//! Raw invocation records are read strictly; a surprise field is refused.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::adapters::local::types::InvocationBytes;
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::InvocationDocument;
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;

/// Why a document was refused.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    UnknownSurprise,
    UnknownOther,
    Malformed,
}

type Failure = Refusal;
type Value = usize;

/// Name a refusal by the unknown field it reports, if any.
fn named(text: &str) -> Failure {
    match text
        .split_once("unknown field `")
        .and_then(|(_, rest)| rest.split_once('`'))
    {
        Some(("surprise", _)) => Refusal::UnknownSurprise,
        Some(_) => Refusal::UnknownOther,
        None => Refusal::Malformed,
    }
}

/// The text of one checked-in fixture document.
fn fixture(path: &str) -> String {
    std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/").to_owned() + path,
    )
    .expect("fixture document")
}


/// The record text a row reads.
type Input = String;

/// A raw invocation record whose process identity was recorded.
fn complete_record(extra: &str) -> String {
    format!(
        "{{{extra}\"actual_request_root\":\"/root\",\"files\":[\"case.py\"],\"executable_sha256\":\"a\",\"settings_sha256\":\"b\",\"normalized_settings\":{{\"arguments\":[\"--check\",\"$ROOT/case.py\"],\"environment\":{{}},\"directory\":\"$ROOT\"}},\"profile_sha256\":\"c\",\"tool\":\"fixture\",\"version\":\"1\",\"state\":\"/state\",\"arguments\":[\"--check\",\"/root/case.py\"],\"environment\":{{}},\"directory\":\"/root\",\"exit\":2,\"signal\":null,\"observer_no_deadline\":true,\"source_sha256\":\"e\"}}"
    )
}

/// A raw invocation record whose capture could not record its process identity.
fn failed_record() -> String {
    "{\"actual_request_root\":\"/root\",\"files\":[\"case.py\"],\"arguments\":[\"--check\"],\"environment\":{},\"directory\":\"/root\",\"exit\":2,\"signal\":null,\"observer_no_deadline\":true,\"metadata_error\":\"identity unavailable\"}".into()
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a raw invocation record with its process identity is read"),
        word: Word::Succeeds,
        input: || complete_record(""),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a raw invocation record of a failed identity capture is read"),
        word: Word::Succeeds,
        input: failed_record,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a raw invocation field the capture never writes is refused by name"),
        word: Word::Refuses,
        input: || complete_record("\"surprise\":1,"),
        expect: Expect::Error(Refusal::UnknownSurprise),
    },
    Case {
        name: CaseName("a raw invocation record without its process identity is refused"),
        word: Word::Refuses,
        input: || complete_record("").replace("\"tool\":\"fixture\",", ""),
        expect: Expect::Error(Refusal::Malformed),
    },
];

/// Decode one record and count its arguments.
fn subject(text: Input) -> Result<Value, Failure> {
    InvocationDocument::try_from(InvocationBytes(&CapturedBytes(text.into_bytes())))
        .map(|record| match record {
            InvocationDocument::Complete(complete) => complete.normalized_settings.arguments.len(),
            InvocationDocument::Failed(failed) => failed.arguments.len(),
        })
        .map_err(|error| named(&error.to_string()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
