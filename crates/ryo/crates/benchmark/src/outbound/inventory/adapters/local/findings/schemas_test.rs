//! An expected finding takes its case's rule when it names none, and one without its end is refused.
use super::finding;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::adapters::local::canonical::schemas::Canonical;
use std::collections::BTreeMap;

/// The finding's table a row reads; its case's rule is `A1`.
type Input = &'static str;
/// The compact text of the normalized finding.
type Value = &'static str;
/// The refusal's message.
type Failure = &'static str;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a finding without a rule takes its case's"),
        word: Word::Succeeds,
        input: || "file = \"a.txt\"\nstart = 0\nend = 1\n",
        expect: Expect::Value("{\"end\":1,\"file\":\"a.txt\",\"rule\":\"A1\",\"start\":0}"),
    },
    Case {
        name: CaseName("a finding without its end is refused"),
        word: Word::Refuses,
        input: || "file = \"a.txt\"\nstart = 0\n",
        expect: Expect::Error("an expected finding names its file, start and end"),
    },
];

/// Reads a TOML table as canonical values.
fn values(document: &str) -> BTreeMap<String, Canonical> {
    let table: toml::Table = toml::from_str(document).expect("a TOML table");
    match Canonical::try_from(toml::Value::Table(table)).expect("a canonical table") {
        Canonical::Table(table) => table,
        Canonical::Null
        | Canonical::Flag(_)
        | Canonical::Integer(_)
        | Canonical::Float(_)
        | Canonical::Text(_)
        | Canonical::List(_) => BTreeMap::new(),
    }
}

/// Normalizes the row's finding.
fn subject(row: Input) -> Result<Value, Failure> {
    let leak = |text: String| -> &'static str { Box::leak(text.into_boxed_str()) };
    finding(&Canonical::Table(values(row)), "A1")
        .and_then(|finding| finding.text())
        .map(leak)
        .map_err(|error| leak(error.to_string()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
