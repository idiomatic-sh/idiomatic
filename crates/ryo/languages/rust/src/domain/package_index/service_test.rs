//! The `links` index's cases: which package holds a file.

use std::collections::{BTreeMap, BTreeSet};

use super::service::{index, package_of};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::paths::types::RepoPath;

/// The file whose package is asked for.
type Input = &'static str;
/// The directory of the package that holds it, or `none`.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_innermost_package_holds_a_file"),
        word: Word::Succeeds,
        input: || "inner/src/lib.rs",
        expect: Expect::Value("inner"),
    },
    Case {
        name: CaseName("a_file_outside_the_inner_package_is_held_by_the_outer"),
        word: Word::Succeeds,
        input: || "src/lib.rs",
        expect: Expect::Value(""),
    },
];

fn subject(file: Input) -> Value {
    let files: BTreeSet<RepoPath> = [
        "Cargo.toml",
        "inner/Cargo.toml",
        "inner/src/lib.rs",
        "src/lib.rs",
    ]
    .iter()
    .map(|each| RepoPath((*each).into()))
    .collect();
    let roots = BTreeMap::from([
        (
            RepoPath(Box::default()),
            BTreeSet::from([RepoPath("src/lib.rs".into())]),
        ),
        (
            RepoPath("inner".into()),
            BTreeSet::from([RepoPath("inner/src/lib.rs".into())]),
        ),
    ]);
    let index = index(&files, &roots);
    package_of(&index, &RepoPath(file.into()))
        .map_or("none", |package| Box::leak(package.0.clone()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
