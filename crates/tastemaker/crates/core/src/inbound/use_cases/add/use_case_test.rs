//! What the `add` use case promises, against the fakes of the ports it calls.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::errors::AddError;
use super::interface::AddUseCase as _;
use super::types::{AddRequest, DefinitionAsked};
use super::use_case::AddUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::files::types::MachinePath;
use crate::domain::paths::types::RepoPath;
use crate::domain::writing::types::Writing;
use crate::inbound::use_cases::check::fixtures::{Workspace, package, repository_of, selection};
use crate::outbound::repository::interface::RepositoryPort as _;

/// The workspace file, the definition asked for, and where.
type Input = (Workspace, &'static str, &'static str, Writing, Problem);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Problem {
    None,
    MissingDependency,
    LockWrite,
    /// The same definition added at the same place twice.
    AddedTwice,
}
/// What the entry is, as `package/name@version`.
type Value = String;
type Failure = AddError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_dry_run_preserves_manifest_and_lock"),
        word: Word::Succeeds,
        input: || {
            (
                Workspace::Placed,
                "part",
                "src/c",
                Writing::DryRun,
                Problem::None,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("unresolved_transitive_assets_refuse_before_writes"),
        word: Word::Refuses,
        input: || {
            (
                Workspace::Placed,
                "part",
                "src/c",
                Writing::Write,
                Problem::MissingDependency,
            )
        },
        expect: Expect::Refusal(|error| selection(matches!(error, AddError::Resolution { .. }))),
    },
    Case {
        name: CaseName("lock_write_failure_reports_partial_state"),
        word: Word::Fails,
        input: || {
            (
                Workspace::Placed,
                "part",
                "src/c",
                Writing::Write,
                Problem::LockWrite,
            )
        },
        expect: Expect::Refusal(|error| selection(matches!(error, AddError::Relock { .. }))),
    },
    Case {
        name: CaseName("a_spec_is_added_at_its_latest_version"),
        word: Word::Succeeds,
        input: || {
            (
                Workspace::Placed,
                "part",
                "src/c",
                Writing::Write,
                Problem::None,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_rule_added_at_the_workspace_reaches_it_through_dot"),
        word: Word::Succeeds,
        input: || {
            (
                Workspace::Placed,
                "p/marker",
                ".",
                Writing::Write,
                Problem::None,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_package_qualified_rule_is_added"),
        word: Word::Succeeds,
        input: || {
            (
                Workspace::Placed,
                "p/marker",
                "src",
                Writing::Write,
                Problem::None,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_spec_added_at_a_capture_writes_no_count"),
        word: Word::Succeeds,
        input: || {
            (
                Workspace::Placed,
                "part",
                "src/{name}",
                Writing::Write,
                Problem::None,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("adding_the_same_entry_twice_writes_it_once"),
        word: Word::Succeeds,
        input: || {
            (
                Workspace::Placed,
                "part",
                "src/c",
                Writing::Write,
                Problem::AddedTwice,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_definition_no_package_holds_is_refused"),
        word: Word::Refuses,
        input: || {
            (
                Workspace::Placed,
                "absent",
                "src",
                Writing::Write,
                Problem::None,
            )
        },
        expect: Expect::Refusal(|actual| {
            if actual
                == &(AddError::NoDefinition {
                    definition: "absent".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_workspace_file_the_manifest_cannot_read_fails_the_add"),
        word: Word::Fails,
        input: || {
            (
                Workspace::Broken,
                "part",
                "src/c",
                Writing::Write,
                Problem::None,
            )
        },
        expect: Expect::Refusal(|error| selection(matches!(error, AddError::Manifest { .. }))),
    },
];

fn subject((workspace, definition, at, writing, problem): Input) -> Result<Value, Failure> {
    let mut repository = repository_of(workspace);
    if problem == Problem::LockWrite {
        let _: bool = repository
            .write_failures
            .insert(RepoPath("tastemaker.lock".into()));
    }
    let files = Arc::new(repository);
    let before = files.files.lock().expect("files").clone();
    let mut package = package();
    if problem == Problem::MissingDependency {
        drop(package.files.insert(
            ("/p".into(), "specs/part/0.1.0/spec.toml".into()),
            crate::inbound::use_cases::check::fixtures::SPEC
                .replacen(
                    "\n[[scenarios]]",
                    "\nrules=[{rule=\"absent/missing@0.1.0\",include=[\"**\"]}]\n\n[[scenarios]]",
                    1,
                )
                .into(),
        ));
    }
    let add = AddUseCaseImpl::new(
        Arc::<crate::outbound::repository::fixtures::fake::RepositoryFake>::clone(&files),
        Arc::new(crate::outbound::manifest::factory::toml()),
        Arc::new(package),
        Arc::new(crate::outbound::matching::factory::globset()),
    );
    let request = AddRequest {
        start: MachinePath("/fake".into()),
        definition: DefinitionAsked(definition.into()),
        at: RepoPath(at.into()),
        params: BTreeMap::new(),
        writing,
    };
    if problem == Problem::AddedTwice {
        let _first = add.add(request.clone())?;
    }
    let result = add.add(request);
    let after = files.files.lock().expect("files").clone();
    if matches!(result, Err(AddError::Relock { .. })) {
        assert_ne!(
            before[&RepoPath("tastemaker.toml".into())],
            after[&RepoPath("tastemaker.toml".into())]
        );
        assert!(!after.contains_key(&RepoPath("tastemaker.lock".into())));
    } else if result.is_err() || writing == Writing::DryRun {
        assert_eq!(before, after);
    } else {
        // A write that succeeded changes the files; the checks below read what it wrote.
    }
    let report = result?;
    if writing == Writing::Write {
        let text = &after[&RepoPath("tastemaker.lock".into())];
        assert!(
            text.0.contains("marker") && text.0.contains("part"),
            "all existing and added releases are locked"
        );
    } else {
        return Ok(String::new());
    }

    // The entry is written once, and with no `count`: the path says how many it places.
    let written = files
        .open(&MachinePath("/fake".into()))
        .read(&RepoPath("tastemaker.toml".into()))
        .is_ok_and(|text| {
            text.0.matches(&format!("\"{at}\"")).count() == 1
                && !text.0.contains("count")
                && !text.0.contains("include = [\"\"]")
        });
    let expected = if definition == "part" {
        "p/part@0.1.0"
    } else {
        "p/marker@0.1.0"
    };
    Ok(if &*report.entry.0 == expected && written {
        String::new()
    } else {
        format!("{} written {written}", report.entry.0)
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
