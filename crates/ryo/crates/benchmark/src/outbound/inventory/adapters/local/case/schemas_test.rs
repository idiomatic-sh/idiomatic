//! A case the authoring registers is read with every field it names.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::adapters::local::types::RegistrationBytes;
use crate::outbound::inventory::types::Registration;
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

/// The authored registration with `change` applied to it.
fn registration_with(change: impl Fn(&mut serde_json::Value)) -> String {
    let mut registration: serde_json::Value =
        serde_json::from_str(&fixture("receipts/python-registration.json")).expect("fixture JSON");
    change(&mut registration);
    registration.to_string()
}

/// Decode one registration and count its cases.
fn subject(text: String) -> Result<Value, Failure> {
    Registration::try_from(RegistrationBytes(&CapturedBytes(text.into_bytes())))
        .map(|registration| registration.rules.iter().map(|rule| rule.cases.len()).sum())
        .map_err(|error| named(&error.to_string()))
}

/// The registration text a row reads.
type Input = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a case of the python registration is read"),
        word: Word::Succeeds,
        input: || fixture("receipts/python-registration.json"),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a case field the authoring never writes is refused by name"),
        word: Word::Refuses,
        input: || {
            registration_with(|registration| {
                registration["rules"][0]["cases"][0]["surprise"] = 1.into();
            })
        },
        expect: Expect::Error(Refusal::UnknownSurprise),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
