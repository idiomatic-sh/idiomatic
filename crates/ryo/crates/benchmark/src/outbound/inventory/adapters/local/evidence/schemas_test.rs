//! A documented difference without evidence is refused; a staged file's digest is the SHA-256 of its bytes.
use super::{evidence, file_digest};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::adapters::local::canonical::schemas::Canonical;
use std::collections::BTreeMap;

/// What a row checks.
#[derive(Clone, Copy, Debug)]
enum Check {
    /// The digest of a staged file holding these bytes.
    Digest(&'static str),
    /// The evidence rows of a language defect, as a TOML table holding `evidence`.
    Rows(&'static str),
}

type Input = Check;
/// The digest, or `checked` for evidence that holds.
type Value = &'static str;
/// The refusal's message.
type Failure = &'static str;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a staged file's digest is the SHA-256 of its bytes"),
        word: Word::Succeeds,
        input: || Check::Digest("x\n"),
        expect: Expect::Value("73cb3858a687a8494ca3323053016282f3dad39d42cf62ca4e79dda2aac7d9ac"),
    },
    Case {
        name: CaseName("a difference without evidence is refused"),
        word: Word::Refuses,
        input: || Check::Rows("evidence = []\n"),
        expect: Expect::Error("reference discrepancy requires independent evidence"),
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

/// Runs the row's check over a fresh stage.
fn subject(check: Input) -> Result<Value, Failure> {
    let leak = |text: String| -> &'static str { Box::leak(text.into_boxed_str()) };
    let stage = tempfile::tempdir().map_err(|error| leak(error.to_string()))?;
    match check {
        Check::Digest(bytes) => {
            let path = stage.path().join("staged");
            std::fs::write(&path, bytes).map_err(|error| leak(error.to_string()))?;
            file_digest(&path).map(leak)
        }
        Check::Rows(document) => evidence(
            stage.path(),
            &BTreeMap::new(),
            "language-defect",
            values(document).get("evidence"),
        )
        .map(|()| "checked"),
    }
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
