//! The `files` service's cases, on values alone.

use super::service::primary_role_from_manifests;
use super::types::{PrimaryFileRole, SourcePath};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// A path and the names of the manifests the language reads.
type Input = (&'static str, &'static [&'static str]);
/// The role the path has.
type Value = PrimaryFileRole;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_named_for_a_manifest_in_a_directory_is_configuration"),
        word: Word::Succeeds,
        input: || ("pkg/member/manifest.toml", &["manifest.toml"]),
        expect: Expect::Value(PrimaryFileRole::Configuration),
    },
    Case {
        name: CaseName("a_file_named_for_no_manifest_is_source"),
        word: Word::Succeeds,
        input: || ("pkg/member/code.src", &["manifest.toml"]),
        expect: Expect::Value(PrimaryFileRole::Source),
    },
    Case {
        name: CaseName("a_language_without_manifests_reads_every_file_as_source"),
        word: Word::Succeeds,
        input: || ("manifest.toml", &[]),
        expect: Expect::Value(PrimaryFileRole::Source),
    },
];

fn subject((path, names): Input) -> Result<Value, Failure> {
    Ok(primary_role_from_manifests(
        SourcePath(path),
        names.iter().copied(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
