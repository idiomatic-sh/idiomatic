//! Documents the python authoring and the native producers write are read by their schemas; unknown fields are refused.
use super::schemas::{ObservationSchema, RegistrationSchema};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::types::{Registration, observation_document::ObservationDocument};

/// Which document a text is read as.
#[derive(Clone, Copy, Debug)]
enum Document {
    Registration,
    Observations,
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

fn refusal(error: &serde_json::Error) -> Failure {
    let text = error.to_string();
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

fn subject((document, text): Input) -> Result<Value, Failure> {
    match document {
        Document::Registration => serde_json::from_str::<RegistrationSchema>(&text)
            .map(Registration::from)
            .map(|registration| registration.rules.iter().map(|rule| rule.cases.len()).sum())
            .map_err(|error| refusal(&error)),
        Document::Observations => text
            .lines()
            .map(|line| {
                serde_json::from_str::<ObservationSchema>(line)
                    .map(ObservationDocument::from)
                    .map_err(|error| refusal(&error))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|rows| rows.len()),
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
