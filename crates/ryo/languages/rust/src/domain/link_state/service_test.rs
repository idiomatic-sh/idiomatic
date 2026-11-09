//! The `link_state` service's cases, on values alone.

use crate::domain::paths::types::Section;
use crate::domain::targets::types::{Checkout, Inheritance};
use std::collections::BTreeSet;

use super::service::located_of;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::conditions::types::PlatformCondition;
use crate::domain::packages::types::{Dependency, Packages, WorkspacePackageMembership};
use crate::domain::paths::types::RepoPath;
use crate::domain::targets::types::BindingName;
use crate::domain::targets::types::PackageName;
use ryo::languages::{FilePath, Optionality};

/// A scenario: what is asked of what the manifests declared, shown.
type Input = fn() -> String;
/// What it came to.
type Value = &'static str;
type Failure = ();

fn declared() -> Packages {
    Packages {
        members: vec![WorkspacePackageMembership {
            workspace: RepoPath("Cargo.toml".into()),
            package: RepoPath("a/Cargo.toml".into()),
        }],
        dependencies: vec![Dependency {
            source: RepoPath("a/Cargo.toml".into()),
            target: Some(RepoPath("b/Cargo.toml".into())),
            origin: ryo::languages::DependencyOrigin::Path,
            binding: BindingName("b".into()),
            package: PackageName("b".into()),
            kind: ryo::languages::DependencyKind::Build,
            condition: PlatformCondition("cfg(unix)".into()),
            optional: Optionality::Optional,
            requirement: Section::Absent,
            checkout: Checkout::default(),
            inheritance: Inheritance::Own,
            replacement: Section::Absent,
        }],
        roots: std::collections::BTreeMap::from([(
            RepoPath("a".into()),
            BTreeSet::from([RepoPath("a/src/lib.rs".into())]),
        )]),
        libraries: std::collections::BTreeMap::new(),
        outputs: vec![RepoPath("target".into())],
        refused: Vec::new(),
    }
}

fn files() -> Vec<FilePath> {
    ["Cargo.toml", "a/Cargo.toml", "b/Cargo.toml"]
        .iter()
        .map(|each| FilePath((*each).into()))
        .collect()
}

fn listed() -> String {
    let located = located_of(&files(), &declared());
    format!(
        "{:?}",
        located
            .listed
            .values()
            .map(|each| each.0)
            .collect::<Vec<_>>()
    )
}

fn packages_declared() -> String {
    (located_of(&files(), &declared()).packages == declared()).to_string()
}

fn unlisted() -> String {
    let located = located_of(&files()[..2], &declared());
    format!(
        "{:?}",
        located
            .listed
            .get(&RepoPath("b/Cargo.toml".into()))
            .map(|each| each.0)
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("what_the_manifests_declared_is_what_the_readers_read"),
        word: Word::Succeeds,
        input: || packages_declared,
        expect: Expect::Value("true"),
    },
    Case {
        name: CaseName("each_manifest_has_the_number_of_its_file"),
        word: Word::Succeeds,
        input: || listed,
        expect: Expect::Value("[0, 1, 2]"),
    },
    Case {
        name: CaseName("a_manifest_the_run_does_not_list_is_numbered_after_the_files"),
        word: Word::Succeeds,
        input: || unlisted,
        expect: Expect::Value("Some(2)"),
    },
];

fn subject(scenario: Input) -> Value {
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Box::leak(scenario().into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
