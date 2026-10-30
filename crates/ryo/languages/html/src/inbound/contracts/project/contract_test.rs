//! The `project` contract's own cases, beside the engine's conformance: the engine's rows run through it, and it names no manifest and refuses none.

use ryo::languages::{self, Project as _, contract};

use super::contract::ProjectContract;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = ();
/// How many manifests it names and how many it refused.
type Value = usize;
type Failure = languages::ProjectError;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("names_no_manifest_and_refuses_none"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(0),
}];

/// Runs the engine's rows against the contract, then asks it to resolve a repository that has no manifest.
fn subject((): Input) -> Result<Value, Failure> {
    let language = ProjectContract::new();
    contract::project(&language);
    let resolved = language.resolve(&languages::ProjectRequest {
        repository: languages::Repository::Disk,
        root: languages::RootPath(Box::default()),
        manifests: Vec::new(),
        files: Vec::new(),
        notes: Vec::new(),
        wanted: Vec::new(),
    })?;
    Ok(language.manifests().len() + resolved.refused.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
