//! The postcard `check_store` adapter: the contract over a temporary directory, then what a store of another build or one
//! that does not read holds, and where it cannot be written.

use super::adapter::CheckStorePostcardAdapter;
use super::constants::{FILE, GITIGNORE};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::files::types::MachinePath;
use crate::domain::settings::types::{Build, Store};
use crate::domain::writing::types::Writing;
use crate::outbound::check_store::errors::CheckStoreError;
use crate::outbound::check_store::fixtures::contract::check;
use crate::outbound::check_store::interface::CheckStorePort as _;
use crate::outbound::check_store::types::Checked;

/// What is done to the store.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Kept in a directory of its own: it holds a `.gitignore` that keeps it all out of version control.
    Ignored,
    /// Read again by its build: it holds what was kept.
    Reopened,
    /// Read again by another build: it holds nothing.
    AnotherBuild,
    /// Overwritten with bytes that are no store: it holds nothing.
    Corrupt,
    /// Kept below a file, where no directory can be made.
    UnderAFile,
    /// Kept under `-n/--dry-run`: no file is written.
    DryRun,
    /// Kept: its file has the mode any file `std::fs::write` makes has, not a temporary file's private one.
    Ordinary,
}

type Input = Scenario;
/// Whether the row holds.
type Value = bool;
type Failure = CheckStoreError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_kept_store_keeps_all_of_itself_out_of_version_control"),
        word: Word::Succeeds,
        input: || Scenario::Ignored,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_store_read_again_by_its_build_holds_what_was_kept"),
        word: Word::Succeeds,
        input: || Scenario::Reopened,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_store_another_build_wrote_holds_nothing"),
        word: Word::Succeeds,
        input: || Scenario::AnotherBuild,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_store_that_does_not_read_holds_nothing"),
        word: Word::Succeeds,
        input: || Scenario::Corrupt,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_store_kept_under_a_dry_run_writes_no_file"),
        word: Word::Succeeds,
        input: || Scenario::DryRun,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_kept_store_has_the_mode_of_an_ordinary_file"),
        word: Word::Succeeds,
        input: || Scenario::Ordinary,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_store_below_a_file_is_not_written"),
        word: Word::Fails,
        input: || Scenario::UnderAFile,
        expect: Expect::Refusal(|error| {
            if matches!(error, CheckStoreError::Unwritable { file, reason } if std::path::Path::new(file).components().any(|part| part.as_os_str() == "file") && !reason.is_empty())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// The adapter of the store in `directory`, under `build`, written.
fn adapter(directory: &std::path::Path, build: &str) -> CheckStorePostcardAdapter {
    writing(directory, build, Writing::Write)
}

/// The adapter of the store in `directory`, under `build`, written as `writing` says.
fn writing(
    directory: &std::path::Path,
    build: &str,
    writing: Writing,
) -> CheckStorePostcardAdapter {
    CheckStorePostcardAdapter::at(&Store {
        directory: MachinePath(directory.display().to_string().into()),
        build: Build(build.into()),
        writing,
    })
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let store = match scenario {
        Scenario::UnderAFile => {
            let file = directory.path().join("file");
            std::fs::write(&file, "").expect("a file");
            file.join("store")
        }
        Scenario::Ignored
        | Scenario::Reopened
        | Scenario::AnotherBuild
        | Scenario::Corrupt
        | Scenario::DryRun
        | Scenario::Ordinary => directory.path().join("store"),
    };
    let answer = Checked {
        evaluated: [crate::domain::findings::service::of_rule(
            crate::domain::findings::types::RuleId("p/a".into()),
        )]
        .into(),
        ..Checked::default()
    };
    let kept = if matches!(scenario, Scenario::DryRun) {
        Writing::DryRun
    } else {
        Writing::Write
    };
    writing(&store, "b", kept).keep_checked(&answer)?;
    Ok(match scenario {
        Scenario::Ignored | Scenario::UnderAFile => std::fs::read_to_string(store.join(GITIGNORE))
            .expect("the store holds a .gitignore")
            .lines()
            .any(|line| line == "*"),
        Scenario::Reopened => adapter(&store, "b").checked() == answer,
        Scenario::DryRun => !store.exists(),
        #[cfg(unix)]
        Scenario::Ordinary => {
            use std::os::unix::fs::PermissionsExt as _;
            let plain = store.join("plain");
            std::fs::write(&plain, "").expect("a plain file");
            let mode = |path: &std::path::Path| {
                std::fs::metadata(path)
                    .expect("a file")
                    .permissions()
                    .mode()
            };
            mode(&store.join(FILE)) == mode(&plain)
        }
        // Where files have no mode, there is none to differ.
        #[cfg(not(unix))]
        Scenario::Ordinary => true,
        Scenario::AnotherBuild => adapter(&store, "other").checked() == Checked::default(),
        Scenario::Corrupt => {
            std::fs::write(store.join(FILE), [0xff_u8; 3]).expect("the store is written");
            adapter(&store, "b").checked() == Checked::default()
        }
    })
}

#[test]
fn succeeds() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    check(&adapter(directory.path(), "b"));
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
