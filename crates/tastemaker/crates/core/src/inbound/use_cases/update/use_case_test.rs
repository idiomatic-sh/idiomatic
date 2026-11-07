//! What the `update` use case promises, against the fakes of the ports it calls.

use std::sync::Arc;

use super::errors::UpdateError;
use super::fixtures::{Problem, package_of, removed, repository_for};
use super::interface::UpdateUseCase as _;
use super::types::{Entries, UpdateRequest, VersionChoice};
use super::use_case::UpdateUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::files::types::MachinePath;
use crate::domain::packages::types::{DefinitionName, VersionText};
use crate::domain::writing::types::Writing;
use crate::inbound::use_cases::check::fixtures::{Workspace, selection};

/// The workspace file, which entries move, to what.
type Input = (
    Workspace,
    Option<&'static str>,
    Option<&'static str>,
    Writing,
    Problem,
);

/// The moves, joined.
type Value = String;
type Failure = UpdateError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_lock_that_is_there_but_cannot_be_read_fails_never_read_as_missing"),
        word: Word::Fails,
        input: || {
            (
                Workspace::Placed,
                Some("marker"),
                Some("0.2.0"),
                Writing::Write,
                Problem::LockUnreadable,
            )
        },
        expect: Expect::Refusal(|error| selection(matches!(error, UpdateError::Repository(_)))),
    },
    Case {
        name: CaseName("unresolved_transitive_assets_refuse_before_writes"),
        word: Word::Refuses,
        input: || {
            (
                Workspace::Placed,
                Some("marker"),
                Some("0.2.0"),
                Writing::Write,
                Problem::MissingDependency,
            )
        },
        expect: Expect::Refusal(|error| selection(matches!(error, UpdateError::Resolution { .. }))),
    },
    Case {
        name: CaseName("lock_write_failure_reports_partial_state"),
        word: Word::Fails,
        input: || {
            (
                Workspace::Placed,
                Some("marker"),
                Some("0.2.0"),
                Writing::Write,
                Problem::LockWrite,
            )
        },
        expect: Expect::Refusal(|error| selection(matches!(error, UpdateError::Relock { .. }))),
    },
    Case {
        name: CaseName("a_valid_target_is_written"),
        word: Word::Succeeds,
        input: || {
            (
                Workspace::Placed,
                Some("marker"),
                Some("0.2.0"),
                Writing::Write,
                Problem::None,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_version_a_move_leaves_stays_in_a_package_in_the_repository"),
        word: Word::Succeeds,
        input: || {
            (
                Workspace::Placed,
                Some("marker"),
                Some("0.2.0"),
                Writing::Write,
                Problem::InRepository,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_valid_target_dry_run_writes_nothing"),
        word: Word::Succeeds,
        input: || {
            (
                Workspace::Placed,
                Some("marker"),
                Some("0.2.0"),
                Writing::DryRun,
                Problem::None,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_absent_target_writes_nothing"),
        word: Word::Refuses,
        input: || {
            (
                Workspace::Placed,
                Some("marker"),
                Some("99.0.0"),
                Writing::Write,
                Problem::None,
            )
        },
        expect: Expect::Refusal(|actual| {
            if actual
                == &(UpdateError::NoVersion {
                    entry: "marker".into(),
                    version: "99.0.0".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_directory_without_definition_writes_nothing"),
        word: Word::Fails,
        input: || {
            (
                Workspace::Placed,
                Some("marker"),
                Some("0.3.0"),
                Writing::Write,
                Problem::None,
            )
        },
        expect: Expect::Refusal(|error| selection(matches!(error, UpdateError::Packages(_)))),
    },
    Case {
        name: CaseName("a_later_entry_failure_writes_nothing"),
        word: Word::Refuses,
        input: || {
            (
                Workspace::Placed,
                None,
                Some("0.2.0"),
                Writing::Write,
                Problem::None,
            )
        },
        expect: Expect::Refusal(|actual| {
            if actual
                == &(UpdateError::NoVersion {
                    entry: "part".into(),
                    version: "0.2.0".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_entry_behind_its_latest_version_moves"),
        word: Word::Succeeds,
        input: || {
            (
                Workspace::Placed,
                None,
                None,
                Writing::DryRun,
                Problem::None,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_entry_moves_to_the_version_asked"),
        word: Word::Succeeds,
        input: || {
            (
                Workspace::Placed,
                Some("marker"),
                Some("0.1.0"),
                Writing::DryRun,
                Problem::None,
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_entry_the_file_has_not_is_refused"),
        word: Word::Refuses,
        input: || {
            (
                Workspace::Placed,
                Some("absent"),
                None,
                Writing::DryRun,
                Problem::None,
            )
        },
        expect: Expect::Refusal(|actual| {
            if actual
                == &(UpdateError::NoEntry {
                    entry: "absent".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_workspace_file_the_manifest_cannot_read_fails_the_update"),
        word: Word::Fails,
        input: || {
            (
                Workspace::Broken,
                None,
                None,
                Writing::DryRun,
                Problem::None,
            )
        },
        expect: Expect::Refusal(|error| selection(matches!(error, UpdateError::Manifest { .. }))),
    },
];

fn subject((workspace, entry, to, writing, problem): Input) -> Result<Value, Failure> {
    let package = package_of(to, problem);
    let repository = repository_for(workspace, problem);
    let before = repository.files.lock().expect("files").clone();
    let update = UpdateUseCaseImpl::new(
        Arc::new(repository.clone()),
        Arc::new(crate::outbound::manifest::factory::toml()),
        Arc::new(package),
        Arc::new(crate::outbound::matching::factory::globset()),
    );
    let result = update.update(UpdateRequest {
        start: MachinePath("/fake".into()),
        entries: entry.map_or(Entries::Every, |name| {
            Entries::Named(DefinitionName(name.into()))
        }),
        target: to.map_or(VersionChoice::Latest, |version| {
            VersionChoice::Version(VersionText(version.into()))
        }),
        writing,
    });
    let after = repository.files.lock().expect("files").clone();
    // The lock pins each entry's version, so `update` never writes the tastemaker.toml.
    assert_eq!(
        before[&crate::domain::paths::types::RepoPath("tastemaker.toml".into())],
        after[&crate::domain::paths::types::RepoPath("tastemaker.toml".into())]
    );
    if matches!(result, Err(UpdateError::Relock { .. })) {
        assert_eq!(
            before[&crate::domain::paths::types::RepoPath("tastemaker.lock".into())],
            after[&crate::domain::paths::types::RepoPath("tastemaker.lock".into())]
        );
    } else if result.is_err() || writing == Writing::DryRun {
        assert_eq!(before, after, "rejected updates and dry runs write nothing");
    } else if to == Some("0.2.0") {
        assert_ne!(before, after, "an update to 0.2.0 changes the files");
        let locked = &after[&crate::domain::paths::types::RepoPath("tastemaker.lock".into())];
        assert!(
            locked.0.contains("0.2.0") && locked.0.contains("part"),
            "the lock pins the release the update moved to"
        );
    } else {
        // An update that moves nothing leaves nothing to compare.
    }
    if problem == Problem::InRepository
        && writing == Writing::Write
        && let Some(left) = removed(&after)
    {
        return Ok(left);
    }
    let report = result?;
    let moves = report
        .moves
        .iter()
        .map(|moved| format!("{} {}>{}", moved.entry.0, moved.from.0, moved.to.0))
        .collect::<Vec<_>>()
        .join(",");
    let expected = if to == Some("0.1.0") {
        ""
    } else {
        "marker 0.1.0>0.2.0"
    };
    Ok(if moves == expected {
        String::new()
    } else {
        moves
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
