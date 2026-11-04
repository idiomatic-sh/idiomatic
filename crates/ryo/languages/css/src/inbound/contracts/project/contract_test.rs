//! What the `project` contract promises: no manifest is the language's, so a repository gives no rows and nothing is refused.

use super::contract::ProjectContract;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::languages::{self, Project as _};

/// Nothing: the contract reads no manifest.
type Input = ();
/// How many manifests it names, and how many it refused.
type Value = (usize, usize);
type Failure = languages::ProjectError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("names_no_manifest_and_refuses_none"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value((0, 0)),
}];

fn subject((): Input) -> Result<Value, Failure> {
    let contract = ProjectContract::new();
    languages::contract::project(&contract);
    let resolved = contract.resolve(&languages::ProjectRequest {
        search_scope: Default::default(),
        given_inventory: Default::default(),
        observed_entries: Vec::new(),
        repository: languages::Repository::Disk,
        root: languages::RootPath("".into()),
        manifests: Vec::new(),
        files: Vec::new(),
        notes: Vec::new(),
        wanted: Vec::new(),
    })?;
    Ok((contract.manifests().len(), resolved.refused.len()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
