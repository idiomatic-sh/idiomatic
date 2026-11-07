//! What the `list` use case promises, against the fakes of the ports it calls.

use std::sync::Arc;

use super::errors::ListError;
use super::interface::ListUseCase as _;
use super::types::{Kinds, ListRequest};
use super::use_case::ListUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::files::types::MachinePath;
use crate::domain::packages::types::DefinitionKind;
use crate::inbound::use_cases::check::fixtures::{Workspace, package, repository_of, selection};

/// The package a row's workspace depends on.
#[derive(Clone, Copy, Debug)]
enum Package {
    /// The check fixtures' package.
    Fixture,
    /// A package whose one rule does not read.
    Broken,
}

/// The workspace file, which kinds, the term, whether the workspace is found, and its package.
type Input = (Workspace, Kinds, &'static str, bool, Package);
/// The names listed, joined.
type Value = String;
type Failure = ListError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_kind_lists_every_definition_at_its_latest_version"),
        word: Word::Succeeds,
        input: || (Workspace::Placed, Kinds::All, "", true, Package::Fixture),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("one_kind_and_a_term_narrow_the_list"),
        word: Word::Succeeds,
        input: || {
            (
                Workspace::Placed,
                Kinds::Only(DefinitionKind::Rule),
                "marker",
                true,
                Package::Fixture,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("relations_are_listed_from_the_catalog_without_a_workspace"),
        word: Word::Succeeds,
        input: || {
            (
                Workspace::Placed,
                Kinds::Relations,
                "",
                false,
                Package::Fixture,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("operators_are_listed_from_the_catalog"),
        word: Word::Succeeds,
        input: || {
            (
                Workspace::Placed,
                Kinds::Operators,
                "",
                true,
                Package::Fixture,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_term_nothing_holds_is_refused"),
        word: Word::Refuses,
        input: || {
            (
                Workspace::Placed,
                Kinds::All,
                "bogus",
                true,
                Package::Fixture,
            )
        },
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ListError::NoMatch {
                    term: "bogus".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("no_workspace_is_refused"),
        word: Word::Refuses,
        input: || (Workspace::Placed, Kinds::All, "", false, Package::Fixture),
        expect: Expect::Refusal(|actual| {
            if actual
                == &(ListError::Repository(
                    crate::outbound::repository::errors::RepositoryError::NoWorkspace {
                        start: "/fake".into(),
                    },
                ))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_workspace_file_the_manifest_cannot_read_fails_the_list"),
        word: Word::Fails,
        input: || (Workspace::Broken, Kinds::All, "", true, Package::Fixture),
        expect: Expect::Refusal(|error| selection(matches!(error, ListError::Manifest { .. }))),
    },
    Case {
        name: CaseName("a_definition_that_does_not_read_fails_the_list_never_listed_blank"),
        word: Word::Fails,
        input: || (Workspace::Placed, Kinds::All, "", true, Package::Broken),
        expect: Expect::Refusal(|error| selection(matches!(error, ListError::Manifest { .. }))),
    },
];

fn subject((workspace, kinds, term, found, held): Input) -> Result<Value, Failure> {
    let mut files = repository_of(workspace);
    if !found {
        files.workspace = crate::outbound::repository::fixtures::fake::FakeWorkspace::Missing;
    }
    let list = ListUseCaseImpl::new(
        Arc::new(files),
        Arc::new(crate::outbound::manifest::factory::toml()),
        Arc::new(match held {
            Package::Fixture => package(),
            Package::Broken => crate::outbound::packages::fixtures::fake::PackagesFake::of(
                "p",
                &[
                    (
                        "package.toml",
                        crate::inbound::use_cases::check::fixtures::PACKAGE,
                    ),
                    ("rules/marker/0.1.0/rule.toml", "version = [\n"),
                ],
            ),
        }),
        Arc::new(crate::outbound::analysis::fixtures::fake::AnalysisFake::default()),
    );
    let report = list.list(ListRequest {
        start: MachinePath("/fake".into()),
        kinds,
        term: super::types::SearchTerm(term.into()),
    })?;
    let names = report
        .listed
        .iter()
        .map(|listed| format!("{}@{}", listed.name.0, listed.version.0))
        .collect::<Vec<_>>()
        .join(" ");
    let expected = match kinds {
        Kinds::Relations => "file@",
        Kinds::Operators => "select@",
        Kinds::All | Kinds::Only(_) if term.is_empty() => "marker@0.1.0 part@0.1.0",
        Kinds::All | Kinds::Only(_) => "marker@0.1.0",
    };
    Ok(if names == expected {
        String::new()
    } else {
        names
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
