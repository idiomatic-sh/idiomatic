//! A case's file-metadata and source digests are the ones Python's authoring computed for the same case; metadata that is
//! not a flag per path is refused.
use super::{file_metadata_digest, source_digest};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::inventory::adapters::local::canonical::schemas::Canonical;
use std::collections::BTreeMap;

/// Which digest a row computes, over which TOML table.
#[derive(Clone, Copy, Debug)]
enum Digest {
    /// The framed digest of the declared file metadata.
    Metadata(&'static str),
    /// The digest of the named sources.
    Sources(&'static str),
}

type Input = Digest;
/// The lowercase hexadecimal digest.
type Value = &'static str;
/// The refusal's message.
type Failure = &'static str;

// The digests are case_records.py's for the case `{ "a.txt" = "x\n", "é.py" = "ü" }` with `file-metadata = { "a.txt" = true }`,
// the same case the authored inventory's test registers.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the file metadata digest is the authoring's"),
        word: Word::Succeeds,
        input: || Digest::Metadata("\"a.txt\" = true\n"),
        expect: Expect::Value("cc3e53679de951bcf1870ff1e6c9743dda8324f9a5dac6366f24e950074e96bf"),
    },
    Case {
        name: CaseName("the source digest is the authoring's"),
        word: Word::Succeeds,
        input: || Digest::Sources("\"a.txt\" = \"x\\n\"\n\"é.py\" = \"ü\"\n"),
        expect: Expect::Value("ea0cf9931b2a3d436a8bf58f577cff3f9ffe39587a67a22289d9115248fa0b28"),
    },
    Case {
        name: CaseName("file metadata that is not a flag is refused"),
        word: Word::Refuses,
        input: || Digest::Metadata("\"a.txt\" = \"yes\"\n"),
        expect: Expect::Error("file metadata must map declared paths to booleans"),
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

/// Computes the row's digest.
fn subject(digest: Input) -> Result<Value, Failure> {
    let leak = |text: String| -> &'static str { Box::leak(text.into_boxed_str()) };
    match digest {
        Digest::Metadata(document) => file_metadata_digest(&values(document)),
        Digest::Sources(document) => source_digest(&values(document)),
    }
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
