//! An authored case registers with the digests Python's authoring computed for it: its whole table, its file metadata
//! and its sources; an isolated profile refuses a case that declares a context.
use super::case_inventory;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::documents::types::text::value::FileMetadataDigest;
use std::borrow::Cow;

/// One case of `A1`: two sources, one beyond ASCII, one expected finding without its rule, and declared file metadata.
const AUTHORED: &str = "[[cases]]\nname = \"clean\"\nfiles = { \"a.txt\" = \"x\\n\", \"é.py\" = \"ü\" }\nexpected = [{ file = \"a.txt\", start = 0, end = 1 }]\nfile-metadata = { \"a.txt\" = true }\n";

/// What the authored stage varies.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// An isolated profile and the case above.
    Isolated,
    /// The same case with a context, under the isolated profile.
    ContextUnderIsolated,
}

type Input = Scenario;
type Value = (Cow<'static, str>, Cow<'static, str>, Cow<'static, str>);
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the case, metadata and source digests are the authoring's"),
        word: Word::Succeeds,
        input: || Scenario::Isolated,
        // Computed by Python's json.dumps(case, sort_keys=True, separators=(',', ':'), ensure_ascii=False), the framed
        // metadata digest and the named source digest of case_records.py.
        expect: Expect::Value((
            Cow::Borrowed("ffa5f30cb40b93ee83bbf5e14155ccb5221072778ff5f109ac6dbbea2c9d8903"),
            Cow::Borrowed("cc3e53679de951bcf1870ff1e6c9743dda8324f9a5dac6366f24e950074e96bf"),
            Cow::Borrowed("ea0cf9931b2a3d436a8bf58f577cff3f9ffe39587a67a22289d9115248fa0b28"),
        )),
    },
    Case {
        name: CaseName("an isolated profile refuses a context-authored case"),
        word: Word::Refuses,
        input: || Scenario::ContextUnderIsolated,
        expect: Expect::Error(Cow::Borrowed("isolated profile cannot admit a context-authored case")),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let stage = tempfile::tempdir().expect("stage");
    std::fs::create_dir_all(stage.path().join("rules/A1")).expect("rule directory");
    std::fs::write(stage.path().join("benchmark.toml"), "defaults = [\"A1\"]\n").expect("profile");
    let cases = match scenario {
        Scenario::Isolated => AUTHORED.to_owned(),
        Scenario::ContextUnderIsolated => format!("{AUTHORED}context = {{ logical-root = \".\" }}\n"),
    };
    std::fs::write(stage.path().join("rules/A1/cases.toml"), cases).expect("cases");
    let documents = case_inventory(stage.path()).map_err(|error| Cow::Owned(error.to_string()))?;
    let [document] = documents.0.as_slice() else {
        panic!("one case is registered");
    };
    let FileMetadataDigest::Declared(metadata) = &document.file_metadata_sha256 else {
        panic!("the metadata digest is written");
    };
    Ok((
        Cow::Owned(document.case_sha256.0.clone()),
        Cow::Owned(metadata.0.clone()),
        Cow::Owned(document.source_sha256.0.clone()),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
