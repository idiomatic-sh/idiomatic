//! The native receipt rows the producers write are read and written back byte-equivalent; a field they never write is refused.
use super::ObservationSchema;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::types::observation_document::ObservationDocument;

type Input = String;
type Value = usize;
type Failure = Refusal;

/// Why a receipt document was refused.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    UnknownField,
    Malformed,
}

fn fixture(path: &str) -> String {
    std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/").to_owned() + path,
    )
    .expect("fixture document")
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every native row is read and written back as the producer wrote it"),
        word: Word::Succeeds,
        input: || fixture("receipts/python-v2-rows.jsonl"),
        expect: Expect::Value(16),
    },
    Case {
        name: CaseName("a row field the producers never write is refused by name"),
        word: Word::Refuses,
        input: || fixture("receipts/python-v2-rows.jsonl").replacen("{", "{\"surprise\":1,", 1),
        expect: Expect::Error(Refusal::UnknownField),
    },
    Case {
        name: CaseName("a row that is not a document is refused"),
        word: Word::Refuses,
        input: || "[1]".into(),
        expect: Expect::Error(Refusal::Malformed),
    },
];

/// The keys a row states: a key written as null and a key left out say the same.
fn present(row: &serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    row.as_object()
        .map(|fields| {
            fields
                .iter()
                .filter(|(_key, value)| !value.is_null())
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// Reads every row through the schema and writes it back to the same document.
fn subject(text: Input) -> Result<Value, Failure> {
    let mut rows = 0;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let original: serde_json::Value = serde_json::from_str(line).map_err(|_| Refusal::Malformed)?;
        let wire: ObservationSchema = serde_json::from_value(original.clone()).map_err(|error| {
            if error.to_string().contains("unknown field") {
                Refusal::UnknownField
            } else {
                Refusal::Malformed
            }
        })?;
        let document = ObservationDocument::from(wire);
        let written = serde_json::to_value(ObservationSchema::from(&document))
            .map_err(|_| Refusal::Malformed)?;
        if present(&written) != present(&original) {
            return Err(Refusal::Malformed);
        }
        rows += 1;
    }
    Ok(rows)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
