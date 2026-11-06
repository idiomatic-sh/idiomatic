//! The `workspaces` service's cases, over repositories held in memory and read by the real manifest and matching adapters.

use super::errors::WorkspacesError;
use super::fixtures::{Files, answering};
use super::service::{excluded, load, members_of, root_of};
use crate::cases::{Matched, Case, CaseName, Expect, Word, run, selects};
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::workspaces::types::Exclusion;

/// What a row of the cases asks of a repository.
#[derive(Clone, Copy)]
enum Scenario {
    /// The workspace of a package below a workspace's directory.
    RootOfMember,
    /// The workspace a package says it is in.
    RootOfNamed,
    /// The workspace of a package no workspace is above.
    RootOfNone,
    /// The members of a workspace named by a pattern, one of them left out.
    Members,
    /// The members of a workspace one of whose members has no manifest.
    MissingMember,
    /// Whether a directory is left out by a workspace that names a member in it.
    ExcludedButNamed,
    /// The members of a workspace whose manifest is not there.
    MissingRoot,
}

/// What a row of the cases asks.
type Input = Scenario;
/// The answer, shown.
type Value = &'static str;
type Failure = WorkspacesError;

const FILES: Files = &[
    (
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\"]\nexclude = [\"crates/old\"]\n",
    ),
    ("crates/a/Cargo.toml", "[package]\nname = \"a\"\n"),
    ("crates/b/Cargo.toml", "[package]\nname = \"b\"\n"),
    ("crates/old/Cargo.toml", "[package]\nname = \"old\"\n"),
    (
        "elsewhere/Cargo.toml",
        "[package]\nname = \"e\"\nworkspace = \"..\"\n",
    ),
    ("lonely/Cargo.toml", "[workspace]\nmembers = [\"gone\"]\n"),
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    fn shown(text: impl Into<Box<str>>) -> &'static str {
        Box::leak(text.into())
    }
    let path = |text: &str| RepoPath(text.into());
    let members = |manifest: &str| {
        answering(FILES, |looking| members_of(looking, &path(manifest))).map(|found| {
            shown(
                found
                    .into_iter()
                    .map(|member| member.0)
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        })
    };
    match scenario {
        Scenario::RootOfMember => Ok(shown(
            answering(FILES, |looking| {
                root_of(looking, &path("crates/a/Cargo.toml"))
            })
            .map_or_else(Box::default, |root| root.0),
        )),
        Scenario::RootOfNamed => Ok(shown(
            answering(FILES, |looking| {
                root_of(looking, &path("elsewhere/Cargo.toml"))
            })
            .map_or_else(Box::default, |root| root.0),
        )),
        Scenario::RootOfNone => Ok(shown(
            answering(FILES, |looking| {
                root_of(looking, &path("crates/old/Cargo.toml"))
            })
            .map_or_else(|| "none".into(), |root| root.0),
        )),
        Scenario::Members => members("Cargo.toml"),
        Scenario::MissingMember => members("lonely/Cargo.toml"),
        Scenario::MissingRoot => members("nowhere/Cargo.toml"),
        Scenario::ExcludedButNamed => {
            let root = answering(FILES, |looking| load(looking, &path("lonely/Cargo.toml")))
                .unwrap_or_default();
            Ok(match root.workspace {
                Section::Present(workspace)
                    if excluded(&workspace, &path("lonely"), &path("lonely/gone"))
                        == Exclusion::Excluded =>
                {
                    "excluded"
                }
                Section::Absent | Section::Present(_) => "kept",
            })
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_package_below_a_workspace_is_in_the_nearest_one"),
        word: Word::Succeeds,
        input: || Scenario::RootOfMember,
        expect: Expect::Value("Cargo.toml"),
    },
    Case {
        name: CaseName("a_package_is_in_the_workspace_it_says_it_is_in"),
        word: Word::Succeeds,
        input: || Scenario::RootOfNamed,
        expect: Expect::Value("Cargo.toml"),
    },
    Case {
        name: CaseName(
            "a_package_with_a_workspace_above_it_is_in_that_one_whether_or_not_it_leaves_it_out",
        ),
        word: Word::Succeeds,
        input: || Scenario::RootOfNone,
        expect: Expect::Value("Cargo.toml"),
    },
    Case {
        name: CaseName("members_named_by_a_pattern_are_found_and_an_excluded_one_is_left_out"),
        word: Word::Succeeds,
        input: || Scenario::Members,
        expect: Expect::Value("crates/a/Cargo.toml crates/b/Cargo.toml"),
    },
    Case {
        name: CaseName("a_directory_a_workspace_names_as_a_member_is_not_left_out"),
        word: Word::Succeeds,
        input: || Scenario::ExcludedButNamed,
        expect: Expect::Value("kept"),
    },
    Case {
        name: CaseName("a_member_that_cannot_be_read_refuses_the_workspace"),
        word: Word::Refuses,
        input: || Scenario::MissingMember,
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(error, WorkspacesError::Member { manifest, reason }
                if &*manifest.0 == "lonely/gone/Cargo.toml" && &*reason.0 == "lonely/gone/Cargo.toml")))
        }),
    },
    Case {
        name: CaseName("a_workspace_whose_manifest_cannot_be_read_is_refused"),
        word: Word::Refuses,
        input: || Scenario::MissingRoot,
        expect: Expect::Refusal(|error| {
            selects(Matched(
                matches!(error, WorkspacesError::Unreadable { manifest, reason }
                if &*manifest.0 == "nowhere/Cargo.toml" && &*reason.0 == "nowhere/Cargo.toml")),
            )
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
