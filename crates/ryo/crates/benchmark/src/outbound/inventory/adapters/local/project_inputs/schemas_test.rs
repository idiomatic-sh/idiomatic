//! A case declares project inputs only when it names search roots or a non-default inventory; roots outside the case and a
//! complete inventory without roots are refused.
use super::project_inputs;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::adapters::local::canonical::schemas::Canonical;
use std::collections::BTreeMap;

/// The case table a row reads; its profile declares nothing.
type Input = &'static str;
/// The compact text of the declared inputs, or `none`.
type Value = &'static str;
/// The refusal's message.
type Failure = &'static str;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a case that declares neither roots nor an inventory has no inputs"),
        word: Word::Succeeds,
        input: || "name = \"clean\"\n",
        expect: Expect::Value("none"),
    },
    Case {
        name: CaseName("declared roots carry the default inventory"),
        word: Word::Succeeds,
        input: || "project-search-roots = [\"src\"]\n",
        expect: Expect::Value(
            "{\"given_project_inventory\":{\"completeness\":\"partial\",\"entries\":[]},\"project_search_roots\":[\"src\"]}",
        ),
    },
    Case {
        name: CaseName("a root outside the case is refused"),
        word: Word::Refuses,
        input: || "project-search-roots = [\"../src\"]\n",
        expect: Expect::Error("project search roots must be ordered safe relative paths"),
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

/// Reads the row's declared project inputs.
fn subject(case: Input) -> Result<Value, Failure> {
    let leak = |text: String| -> &'static str { Box::leak(text.into_boxed_str()) };
    match project_inputs(&values(case), &BTreeMap::new())
        .map_err(|error| leak(error.to_string()))?
    {
        None => Ok("none"),
        Some(inputs) => serde_json::to_string(&inputs)
            .map(leak)
            .map_err(|error| leak(error.to_string())),
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
