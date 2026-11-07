//! The git adapter of the repository port: the contract over a repository made in a temporary directory, a revision
//! it does not hold, a directory that is no repository, a revision git cannot read and an ignored input declared.

use std::process::Command;

use super::adapter::RepositoryGitAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::manifest::types::{Pathspec, ProductRoot};
use crate::domain::operations::types::Revision;
use crate::domain::renames::types::{Head, HistoryStart};
use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::repository::fixtures::contract::check;
use crate::outbound::repository::interface::RepositoryPort as _;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The contract.
    Contract,
    /// A history after a revision the clone does not hold.
    Unknown,
    /// The files of a directory that is no repository.
    NotARepository,
    /// The renames from a revision git cannot read.
    Failed,
    /// A file moved with `mv`, not added.
    Untracked,
    /// An input git ignores, declared by its own path, beside an ignored directory no input names.
    IgnoredInput,
}

type Input = Question;
/// Whether the answer is the expected one.
type Value = bool;
type Failure = RepositoryError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_adapter_keeps_the_contract"),
        word: Word::Succeeds,
        input: || Question::Contract,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_file_moved_and_not_added_is_a_rename"),
        word: Word::Succeeds,
        input: || Question::Untracked,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "an_ignored_input_declared_by_its_path_is_listed_and_nothing_else_ignored_is",
        ),
        word: Word::Succeeds,
        input: || Question::IgnoredInput,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_history_after_a_revision_this_clone_lacks_is_refused"),
        word: Word::Refuses,
        input: || Question::Unknown,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(RepositoryError::UnknownRevision {
                    revision: "ffffffffffffffffffffffffffffffffffffffff".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_directory_that_is_no_repository_is_refused"),
        word: Word::Refuses,
        input: || Question::NotARepository,
        expect: Expect::Refusal(|error| {
            if matches!(error, RepositoryError::NotARepository {location} if std::path::Path::new(location).is_absolute() && std::path::Path::new(location).file_name().is_some_and(|name| name.to_string_lossy().starts_with(".tmp")))
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_revision_git_cannot_read_fails"),
        word: Word::Fails,
        input: || Question::Failed,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(RepositoryError::Failed {
                    reason:
                        "fatal: option '--not-a-revision' must come before non-option arguments"
                            .into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// Runs git in `directory`, with an identity of its own.
fn git(directory: &std::path::Path, arguments: &[&str]) {
    let status = Command::new("git")
        .args([
            "-c",
            "user.name=asymptote",
            "-c",
            "user.email=asymptote@example.com",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(arguments)
        .current_dir(directory)
        .env_remove("GIT_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_WORK_TREE")
        .output()
        .expect("git runs");
    assert!(
        status.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

/// A repository whose first commit adds `a.rs` and an operation, whose second renames `a.rs` to `b.rs`, and whose
/// working tree renames `b.rs` to `c.rs`.
fn repository(path: &std::path::Path) {
    git(path, &["init", "-q", "-b", "main"]);
    std::fs::write(path.join("a.rs"), "fn a() {}\nfn b() {}\nfn c() {}\n").expect("a.rs");
    let lock = path.join(".asymptote/operations");
    std::fs::create_dir_all(&lock).expect("the lock");
    std::fs::write(
        lock.join(format!("{}.lock", "a".repeat(64))),
        "version = 1\n",
    )
    .expect("an operation");
    git(path, &["add", "."]);
    git(path, &["commit", "-q", "-m", "first"]);
    git(path, &["mv", "a.rs", "b.rs"]);
    git(path, &["commit", "-q", "-m", "second"]);
    git(path, &["mv", "b.rs", "c.rs"]);
}

/// Answers one row's question about the git adapter over a fresh temporary repository.
fn subject(question: Input) -> Result<Value, Failure> {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = ProductRoot(directory.path().display().to_string().into());
    if !matches!(question, Question::NotARepository) {
        repository(directory.path());
    }
    // The working tree is listed when the adapter is made, after the row has written its files, as an edge lists it.
    let git = || {
        let lister = RepositoryGitAdapter::new(
            root.clone(),
            Ok(crate::outbound::repository::types::Worktree::default()),
        );
        RepositoryGitAdapter::new(root.clone(), lister.worktree())
    };
    let answer = match question {
        Question::Contract => {
            check(&git());
            Ok(true)
        }
        Question::Unknown => git()
            .rename_history(
                &HistoryStart::After(Revision("f".repeat(40).into())),
                &Revision("HEAD".into()),
            )
            .map(|_| true),
        Question::NotARepository => {
            let head = git().head();
            assert_eq!(
                head,
                Ok(Head::Detached),
                "no repository stands on no commit"
            );
            git().files(&[Pathspec(".".into())]).map(|_| true)
        }
        Question::Untracked => {
            std::fs::rename(directory.path().join("c.rs"), directory.path().join("d.rs"))
                .expect("a move");
            git()
                .renames_to_worktree(&Revision("HEAD".into()))
                .map(|renames| {
                    renames
                        .0
                        .iter()
                        .any(|rename| &*rename.from.0 == "b.rs" && &*rename.to.0 == "d.rs")
                })
        }
        Question::Failed => git()
            .renames_to_worktree(&Revision("--not-a-revision".into()))
            .map(|_| true),
        Question::IgnoredInput => {
            let path = directory.path();
            std::fs::write(path.join(".gitignore"), "generated.sarif\ntarget/\n").expect("ignores");
            std::fs::write(path.join("generated.sarif"), "{}\n").expect("an ignored report");
            std::fs::create_dir_all(path.join("target")).expect("target");
            std::fs::write(path.join("target/x"), "built\n").expect("a build output");
            let listed = |specs: &[&str]| {
                let specs: Vec<Pathspec> =
                    specs.iter().map(|spec| Pathspec((*spec).into())).collect();
                let ignored = git().ignored(&specs)?;
                git().files(&specs).map(|files| {
                    files
                        .0
                        .iter()
                        .chain(ignored.0.values().flatten())
                        .map(|file| file.0.to_string())
                        .collect::<Vec<_>>()
                })
            };
            let declared = listed(&[".", "generated.sarif"])?;
            let plain = listed(&["."])?;
            Ok(declared.iter().any(|file| file == "generated.sarif")
                && !declared.iter().any(|file| file == "target/x")
                && !plain.iter().any(|file| file == "generated.sarif"))
        }
    };
    if let Err(error) = &answer {
        assert!(matches!(
            error,
            RepositoryError::UnknownRevision { .. }
                | RepositoryError::NotARepository { .. }
                | RepositoryError::Failed { .. }
        ));
    }
    answer
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
