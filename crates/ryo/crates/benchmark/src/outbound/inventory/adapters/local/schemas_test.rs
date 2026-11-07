//! Documents the python authoring and the native producers write are read by their schemas; unknown fields are refused.
use super::schemas::{RegistrationSchema, invocation};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::types::{InvocationDocument, Registration};
use crate::outbound::suite::{interface::SuitePort as _, types::ArtifactPath};

/// Which document a text is read as.
#[derive(Clone, Copy, Debug)]
enum Document {
    Registration,
    Observations,
    Invocation,
}

type Input = (Document, String);
type Value = usize;

/// Why a document was refused.
#[derive(Debug, PartialEq, Eq)]
enum Failure {
    UnknownSurprise,
    UnknownOther,
    Malformed,
}

fn fixture(path: &str) -> String {
    std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/").to_owned() + path,
    )
    .expect("fixture document")
}

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

fn python_registration_with(extra: &str) -> String {
    fixture("receipts/python-registration.json").replacen(
        "\"case_sha256\"",
        &format!("{extra}\"case_sha256\""),
        1,
    )
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a registration the python authoring wrote is read with every case field"),
        word: Word::Succeeds,
        input: || {
            (
                Document::Registration,
                fixture("receipts/python-registration.json"),
            )
        },
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the receipt-only registration fixture is read"),
        word: Word::Succeeds,
        input: || {
            (
                Document::Registration,
                fixture("registrations/receipt_only/registration.json"),
            )
        },
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("receipt rows the native producers wrote are read"),
        word: Word::Succeeds,
        input: || {
            (
                Document::Observations,
                fixture("receipts/python-v2-rows.jsonl"),
            )
        },
        expect: Expect::Value(16),
    },
    Case {
        name: CaseName("the receipt-only observation fixture is read"),
        word: Word::Succeeds,
        input: || {
            (
                Document::Observations,
                fixture("registrations/receipt_only/observations.jsonl"),
            )
        },
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a raw invocation record with its process identity is read"),
        word: Word::Succeeds,
        input: || (Document::Invocation, complete_record("")),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a raw invocation record of a failed identity capture is read"),
        word: Word::Succeeds,
        input: || (Document::Invocation, failed_record()),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a raw invocation field the capture never writes is refused by name"),
        word: Word::Refuses,
        input: || (Document::Invocation, complete_record("\"surprise\":1,")),
        expect: Expect::Error(Failure::UnknownSurprise),
    },
    Case {
        name: CaseName("a raw invocation record without its process identity is refused"),
        word: Word::Refuses,
        input: || {
            (
                Document::Invocation,
                complete_record("").replace("\"tool\":\"fixture\",", ""),
            )
        },
        expect: Expect::Error(Failure::Malformed),
    },
    Case {
        name: CaseName("a case field the authoring never writes is refused by name"),
        word: Word::Refuses,
        input: || {
            (
                Document::Registration,
                python_registration_with("\"surprise\": 1, "),
            )
        },
        expect: Expect::Error(Failure::UnknownSurprise),
    },
    Case {
        name: CaseName("a receipt row field the producers never write is refused by name"),
        word: Word::Refuses,
        input: || {
            (
                Document::Observations,
                fixture("receipts/python-v2-rows.jsonl").replacen("{", "{\"surprise\":1,", 1),
            )
        },
        expect: Expect::Error(Failure::UnknownSurprise),
    },
    Case {
        name: CaseName("a receipt row that is not a document is refused"),
        word: Word::Refuses,
        input: || (Document::Observations, "[1]".into()),
        expect: Expect::Error(Failure::Malformed),
    },
];

fn unknown_field(text: &str) -> Failure {
    text.split_once("unknown field `")
        .and_then(|(_, rest)| rest.split_once('`'))
        .map_or(Failure::Malformed, |(name, _)| {
            if name == "surprise" {
                Failure::UnknownSurprise
            } else {
                Failure::UnknownOther
            }
        })
}

/// The refusal of a decoded invocation record, named as the schema refusals are.
fn named(text: &str) -> Failure {
    match text
        .split_once("unknown field `")
        .and_then(|(_, rest)| rest.split_once('`'))
    {
        Some(("surprise", _)) => Failure::UnknownSurprise,
        Some(_) => Failure::UnknownOther,
        None => Failure::Malformed,
    }
}

fn subject((document, text): Input) -> Result<Value, Failure> {
    match document {
        Document::Invocation => invocation(text.as_bytes())
            .map(|record| match record {
                InvocationDocument::Complete(complete) => {
                    complete.normalized_settings.arguments.len()
                }
                InvocationDocument::Failed(failed) => failed.arguments.len(),
            })
            .map_err(|error| named(&error)),
        Document::Registration => serde_json::from_str::<RegistrationSchema>(&text)
            .map(Registration::from)
            .map(|registration| registration.rules.iter().map(|rule| rule.cases.len()).sum())
            .map_err(|error| unknown_field(&error.to_string())),
        Document::Observations => {
            let directory = tempfile::tempdir().expect("receipt directory");
            let receipt = directory.path().join("cases.jsonl");
            std::fs::write(&receipt, text).expect("receipt rows");
            crate::outbound::suite::factory::local()
                .case_observations(ArtifactPath(&receipt))
                .map(|rows| rows.len())
                .map_err(|error| unknown_field(&error.to_string()))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
