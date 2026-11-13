//! A case without a documented difference registers none, and a case that names two authorities for one is refused.
use super::reference_discrepancy;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::adapters::local::canonical::schemas::Canonical;
use std::collections::BTreeMap;

/// The case table a row reads; its profile declares nothing.
type Input = &'static str;
/// The compact text of the registered difference.
type Value = &'static str;
/// The refusal's message.
type Failure = &'static str;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a case without a documented difference registers none"),
        word: Word::Succeeds,
        input: || "name = \"clean\"\n",
        expect: Expect::Value("null"),
    },
    Case {
        name: CaseName("a case naming two authorities is refused"),
        word: Word::Refuses,
        input: || "reference-discrepancy = {}\ntool-contract-difference = {}\n",
        expect: Expect::Error("a case cannot declare two discrepancy authorities"),
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

/// Registers the row's documented difference.
fn subject(case: Input) -> Result<Value, Failure> {
    let leak = |text: String| -> &'static str { Box::leak(text.into_boxed_str()) };
    let stage = tempfile::tempdir().map_err(|error| leak(error.to_string()))?;
    reference_discrepancy(stage.path(), &BTreeMap::new(), &values(case))
        .and_then(|difference| difference.text())
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
