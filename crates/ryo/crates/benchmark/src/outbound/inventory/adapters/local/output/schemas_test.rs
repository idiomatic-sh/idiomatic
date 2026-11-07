//! Receipt rows the native producers wrote are read; the summary and rows this adapter writes are exact.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::adapters::local::types::{RowJson, SummaryJson};
use crate::outbound::inventory::fixtures::data::{row, summary};

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


/// What a row feeds the document: receipt rows to read, or a summary or row to write.
#[derive(Debug)]
enum Subject {
    Observations(String),
    Summary,
    Row,
}

/// What a row of this table gives the subject.
type Input = Subject;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("receipt rows the native producers wrote are read"),
        word: Word::Succeeds,
        input: || Subject::Observations(fixture("receipts/python-v2-rows.jsonl")),
        expect: Expect::Value(16),
    },
    Case {
        name: CaseName("the receipt-only observation fixture is read"),
        word: Word::Succeeds,
        input: || Subject::Observations(fixture("registrations/receipt_only/observations.jsonl")),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a summary is written with its schema, state and counts"),
        word: Word::Succeeds,
        input: || Subject::Summary,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a row is written with its explicit nulls"),
        word: Word::Succeeds,
        input: || Subject::Row,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a receipt row field the producers never write is refused by name"),
        word: Word::Refuses,
        input: || {
            Subject::Observations(
                fixture("receipts/python-v2-rows.jsonl").replacen("{", "{\"surprise\":1,", 1),
            )
        },
        expect: Expect::Error(Refusal::UnknownSurprise),
    },
    Case {
        name: CaseName("a receipt row that is not a document is refused"),
        word: Word::Refuses,
        input: || Subject::Observations("[1]".into()),
        expect: Expect::Error(Refusal::Malformed),
    },
];

/// Read the rows of a receipt, or write a summary or row and check what it says.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Subject::Observations(text) => {
            let directory = tempfile::tempdir().expect("receipt directory");
            let receipt = directory.path().join("cases.jsonl");
            std::fs::write(&receipt, text).expect("receipt rows");
            use crate::outbound::receipts::interface::ReceiptsPort as _;
            crate::outbound::receipts::factory::local(
                crate::outbound::receipts::types::CaseReceipts::Unrecorded,
            )
            .case_observations(crate::outbound::receipts::types::ReceiptPath(&receipt))
            .map(|rows| rows.len())
            .map_err(|error| named(&error.to_string()))
        }
        Subject::Summary => {
            let bytes = SummaryJson::try_from(&summary()).map_err(|error| named(&error.to_string()))?;
            let value: serde_json::Value =
                serde_json::from_slice(&bytes.0.0).expect("a summary is JSON");
            Ok(usize::from(
                value["schema"] == "benchmark-inventory/v1"
                    && value["run_state"] == "running"
                    && value["counts"] == serde_json::json!({"PASS": 0, "FAIL": 1}),
            ))
        }
        Subject::Row => {
            let bytes = RowJson::try_from(&row()).map_err(|error| named(&error.to_string()))?;
            let value: serde_json::Value = serde_json::from_slice(&bytes.0.0).expect("a row is JSON");
            Ok(usize::from(
                value["status"] == "FAIL"
                    && value["reason"] == "missing"
                    && value["instance"].is_null()
                    && value["case_results_sha256"].is_null(),
            ))
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
