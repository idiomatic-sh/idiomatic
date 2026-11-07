//! What every `repository` adapter must pass, over a repository it may write to, bound to its root: files written are
//! read, listed, walked and stamped, a workspace written is found, a scratch directory is made and removed when dropped,
//! and two repositories bound to two roots never see each other's files.

// Error locations are contractual; diagnostic wording belongs to each implementation.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::files::types::{FileText, MachinePath, Stamp};
use crate::domain::paths::types::RepoPath;
use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::repository::interface::RepositoryPort;
use crate::outbound::repository::types::Presence;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
pub enum Question {
    /// A file written is read back.
    WrittenIsRead,
    /// A file written is listed under its directory.
    WrittenIsListed,
    /// A file written is walked, and so is its directory.
    WrittenIsWalked,
    /// A file never written is absent.
    NeverWrittenIsAbsent,
    /// A file written and removed is absent, and so is its directory once it is removed too.
    RemovedIsAbsent,
    /// A file never written cannot be read.
    NeverWrittenIsNotRead,
    /// A file below a file cannot be written.
    BelowAFileIsNotWritten,
    /// A workspace written at the root is found there.
    WorkspaceIsFound,
    /// The repository the root is in is found.
    RepositoryIsFound,
    /// A file's stamp moves when the file changes, and a file never written has the empty stamp.
    StampsMove,
    /// A project is made once at its root, and one dropped holds nothing after.
    ScratchIsNew,
    /// It says it skips some directories, sorted, each named and each declared by someone.
    SkippedAreDeclared,
    /// A directory opened is bound at the path it was asked for.
    OpenedIsBound,
    /// A file written through one project is neither seen nor read through another, bound to another root.
    RootsAreApart,
}

/// What a row asks.
type Input = Question;
/// Whether the answer is the one the contract expects.
type Value = bool;
/// The port's own failure.
type Failure = RepositoryError;

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_written_is_read_back"),
        word: Word::Succeeds,
        input: || Question::WrittenIsRead,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_file_written_is_listed_under_its_directory"),
        word: Word::Succeeds,
        input: || Question::WrittenIsListed,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_file_written_is_walked_with_its_directory"),
        word: Word::Succeeds,
        input: || Question::WrittenIsWalked,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_file_never_written_is_absent"),
        word: Word::Succeeds,
        input: || Question::NeverWrittenIsAbsent,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_file_removed_is_absent_and_so_is_its_directory_once_removed"),
        word: Word::Succeeds,
        input: || Question::RemovedIsAbsent,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_file_never_written_cannot_be_read"),
        word: Word::Refuses,
        input: || Question::NeverWrittenIsNotRead,
        expect: Expect::Refusal(|error| {
            if matches!(error, RepositoryError::Read {path,reason} if path == "contract/never.txt" && !reason.is_empty())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_file_below_a_file_cannot_be_written"),
        word: Word::Fails,
        input: || Question::BelowAFileIsNotWritten,
        expect: Expect::Refusal(|error| {
            if matches!(error, RepositoryError::Write {path,reason} if path == "contract/a.txt/below.txt" && !reason.is_empty())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_workspace_written_at_the_root_is_found_there"),
        word: Word::Succeeds,
        input: || Question::WorkspaceIsFound,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_repository_of_the_root_is_found"),
        word: Word::Succeeds,
        input: || Question::RepositoryIsFound,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_stamp_moves_with_its_file"),
        word: Word::Succeeds,
        input: || Question::StampsMove,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("each_case_directory_is_a_new_one_and_gone_once_dropped"),
        word: Word::Succeeds,
        input: || Question::ScratchIsNew,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_skipped_directories_are_sorted_and_each_has_who_declares_it"),
        word: Word::Succeeds,
        input: || Question::SkippedAreDeclared,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_directory_opened_is_bound_at_the_path_asked_for"),
        word: Word::Succeeds,
        input: || Question::OpenedIsBound,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("two_repositories_bound_to_two_roots_do_not_see_each_others_files"),
        word: Word::Succeeds,
        input: || Question::RootsAreApart,
        expect: Expect::Value(true),
    },
];

/// Whether `adapter` answers `question` as the contract expects, over the repository at `root`.
fn answer(
    adapter: &impl RepositoryPort,
    root: &MachinePath,
    question: Question,
) -> Result<Value, Failure> {
    let path = RepoPath("contract/a.txt".into());
    let text = FileText("contract\n".into());
    let bound = adapter.open(root);
    let repository = &*bound;
    match question {
        Question::WrittenIsRead => {
            repository.write(&path, &text)?;
            repository.write(&path, &text)?;
            Ok(repository.read(&path)? == text)
        }
        Question::WrittenIsListed => {
            repository.write(&path, &text)?;
            Ok(repository
                .list(&RepoPath("contract".into()))?
                .contains(&path))
        }
        Question::WrittenIsWalked => {
            repository.write(&path, &text)?;
            let walk = repository.walk(&RepoPath("contract".into()))?;
            Ok(walk.files.contains(&path)
                && walk.directories.contains(&RepoPath("contract".into())))
        }
        Question::NeverWrittenIsAbsent => {
            Ok(repository.exists(&RepoPath("contract/never.txt".into())) == Presence::Absent)
        }
        Question::RemovedIsAbsent => {
            let directory = RepoPath("contract/removed".into());
            let file = RepoPath("contract/removed/a.txt".into());
            repository.write(&file, &text)?;
            repository.remove(&file)?;
            let file_gone = repository.exists(&file) == Presence::Absent;
            if repository.exists(&directory) == Presence::Present {
                repository.remove(&directory)?;
            }
            Ok(file_gone && repository.exists(&directory) == Presence::Absent)
        }
        Question::NeverWrittenIsNotRead => repository
            .read(&RepoPath("contract/never.txt".into()))
            .map(|_| true),
        Question::BelowAFileIsNotWritten => {
            repository.write(&path, &text)?;
            repository
                .write(&RepoPath("contract/a.txt/below.txt".into()), &text)
                .map(|()| true)
        }
        Question::WorkspaceIsFound => {
            repository.write(&RepoPath("tastemaker.toml".into()), &FileText("".into()))?;
            let found = adapter.find(root)?;
            Ok(*found.workspace() == RepoPath::default()
                && *found.start() == RepoPath::default()
                && adapter
                    .open(found.root())
                    .exists(&RepoPath("tastemaker.toml".into()))
                    == Presence::Present)
        }
        Question::RepositoryIsFound => adapter.repository(root).map(|_| true),
        Question::StampsMove => {
            let never = RepoPath("contract/never.txt".into());
            repository.write(&path, &text)?;
            let before = repository.stamps(&[path.clone(), never.clone()]);
            repository.write(&path, &FileText("contract, longer\n".into()))?;
            let after = repository.stamps(&[path.clone(), never]);
            Ok(match (before.0.as_slice(), after.0.as_slice()) {
                ([was, _], [now, absent]) => {
                    was != now && *absent == Stamp::default() && *now != Stamp::default()
                }
                _ => false,
            })
        }
        Question::SkippedAreDeclared => {
            let skipped = adapter.skipped().0;
            Ok(!skipped.is_empty()
                && skipped.is_sorted()
                && skipped
                    .iter()
                    .all(|each| !each.name.0.is_empty() && !each.by.is_empty()))
        }
        Question::ScratchIsNew => scratch_is_new(adapter, root),
        Question::OpenedIsBound => Ok(repository.root() == root),
        Question::RootsAreApart => roots_are_apart(adapter, root),
    }
}

/// Runs the contract's rows against one implementation, over the repository at `root`.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl RepositoryPort, root: &MachinePath) {
    run(CASES, Word::Succeeds, |question| {
        answer(adapter, root, question)
    });
    run(CASES, Word::Refuses, |question| {
        answer(adapter, root, question)
    });
    run(CASES, Word::Fails, |question| {
        answer(adapter, root, question)
    });
}

/// Whether a scratch repository is new: held while it lives, taken by no second scratch, and gone once dropped.
fn scratch_is_new(adapter: &impl RepositoryPort, root: &MachinePath) -> Result<Value, Failure> {
    let (first, second) = (
        MachinePath(format!("{}-first", root.0).into()),
        MachinePath(format!("{}-second", root.0).into()),
    );
    let (one, _other) = (adapter.project(&first)?, adapter.project(&second)?);
    let file = RepoPath("kept.txt".into());
    one.write(&file, &FileText("kept\n".into()))?;
    let held = one.exists(&file) == Presence::Present;
    let taken = adapter.project(&second).is_err();
    drop(one);
    Ok(held
        && taken
        && adapter.open(&first).exists(&file) == Presence::Absent
        && adapter.project(&first).is_ok())
}

/// Whether two repositories bound to two roots never see each other's files.
fn roots_are_apart(adapter: &impl RepositoryPort, root: &MachinePath) -> Result<Value, Failure> {
    let (one, other) = (
        adapter.project(&MachinePath(format!("{}-one", root.0).into()))?,
        adapter.project(&MachinePath(format!("{}-other", root.0).into()))?,
    );
    let file = RepoPath("apart/a.txt".into());
    one.write(&file, &FileText("one\n".into()))?;
    other.write(&RepoPath("apart/b.txt".into()), &FileText("other\n".into()))?;
    let unseen = other.exists(&file) == Presence::Absent && other.read(&file).is_err();
    let unlisted = !other.list(&RepoPath("apart".into()))?.contains(&file)
        && !other.walk(&RepoPath("apart".into()))?.files.contains(&file);
    Ok(unseen && unlisted && one.read(&file)? == FileText("one\n".into()))
}
