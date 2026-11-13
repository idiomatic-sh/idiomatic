//! An authored TOML value becomes the canonical value the authoring's JSON writes: keys sorted, compact, characters beyond ASCII
//! kept; a value JSON cannot hold is refused.
use super::Canonical;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// The TOML document a row reads.
type Input = &'static str;
/// The compact text of its canonical value.
type Value = &'static str;
/// The refusal's message.
type Failure = &'static str;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a document is written with its keys sorted and its text unescaped"),
        word: Word::Succeeds,
        input: || "b = [1, 2.5, true]\na = \"é\"\n",
        expect: Expect::Value("{\"a\":\"é\",\"b\":[1,2.5,true]}"),
    },
    Case {
        name: CaseName("a date the authoring cannot write is refused"),
        word: Word::Refuses,
        input: || "a = 1979-05-27\n",
        expect: Expect::Error("a profile or case holds a value the registration cannot represent"),
    },
];

/// Reads the row's document as a canonical value and writes it.
fn subject(document: Input) -> Result<Value, Failure> {
    let leak = |text: String| -> &'static str { Box::leak(text.into_boxed_str()) };
    let table: toml::Table = toml::from_str(document).map_err(|error| leak(error.to_string()))?;
    Canonical::try_from(toml::Value::Table(table))
        .and_then(|value| value.text())
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
