//! What the `scopes` service answers of the files a directory holds.

use super::service::holds_file;
use super::types::FileHolding;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::paths::types::RepoPath;

/// The file of the repository, and the directory asked about.
type Input = (&'static str, &'static str);
/// Whether the directory holds a file directly.
type Value = FileHolding;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_directly_in_a_directory_is_held"),
        word: Word::Succeeds,
        input: || ("a/c.txt", "a"),
        expect: Expect::Value(FileHolding::HoldsFiles),
    },
    Case {
        name: CaseName("a_file_beneath_a_directory_is_not_held_by_it"),
        word: Word::Succeeds,
        input: || ("a/b/c.txt", "a"),
        expect: Expect::Value(FileHolding::HoldsNone),
    },
];

fn subject((file, directory): Input) -> Value {
    holds_file(&[RepoPath(file.into())], &RepoPath(directory.into()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
