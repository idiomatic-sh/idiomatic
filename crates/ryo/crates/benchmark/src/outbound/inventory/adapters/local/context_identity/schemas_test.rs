//! An isolated case has no context identity, and an isolated profile refuses a case that declares one; a listed-disk profile
//! refuses a case without its context.
use super::context_identity;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::adapters::local::canonical::schemas::Canonical;
use std::collections::BTreeMap;

/// The case table a row reads and the profile's context mode.
type Input = (&'static str, &'static str);
/// The compact text of the context identity.
type Value = &'static str;
/// The refusal's message.
type Failure = &'static str;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an isolated case has no context identity"),
        word: Word::Succeeds,
        input: || ("name = \"clean\"\n", "isolated"),
        expect: Expect::Value("null"),
    },
    Case {
        name: CaseName("an isolated profile refuses a case with a context"),
        word: Word::Refuses,
        input: || ("context = { logical-root = \".\" }\n", "isolated"),
        expect: Expect::Error("isolated profile cannot admit a context-authored case"),
    },
    Case {
        name: CaseName("a listed-disk profile refuses a case without its context"),
        word: Word::Refuses,
        input: || ("name = \"clean\"\n", "listed-disk"),
        expect: Expect::Error("listed-disk profile requires an explicit context for every case"),
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

/// Frames the row's context identity.
fn subject((case, mode): Input) -> Result<Value, Failure> {
    let leak = |text: String| -> &'static str { Box::leak(text.into_boxed_str()) };
    context_identity(&values(case), mode)
        .and_then(|identity| identity.text())
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
