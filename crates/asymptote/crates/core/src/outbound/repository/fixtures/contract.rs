//! What every `repository` adapter must pass, over a repository whose first commit adds `a.rs` and the lock's
//! operation `.asymptote/operations/<64 a>.lock`, whose second renames `a.rs` to `b.rs`, and whose working tree
//! renames `b.rs` to `c.rs`.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::manifest::types::Pathspec;
use crate::domain::operations::types::Revision;
use crate::domain::renames::types::{Head, HistoryStart};
use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::repository::interface::RepositoryPort;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
pub enum Question {
    /// The renames from the first commit on.
    History,
    /// The renames to the working tree.
    Worktree,
    /// The commits that added the lock's files.
    Introductions,
    /// The lock's files at the head.
    ReadAt,
    /// The files the product holds.
    Files,
    /// The ignored inputs among the product's pathspecs.
    Ignored,
    /// The merge base of the head with itself.
    MergeBase,
    /// A revision no commit has.
    Unknown,
    /// `HEAD`, resolved to a commit.
    Resolved,
    /// A revision no commit has, resolved.
    UnknownCommit,
}

/// What the answer comes to.
type Input = Question;
/// The answer as text, borrowed when a row writes it in a constant.
type Value = std::borrow::Cow<'static, str>;
/// The repository's error, which a refusal row expects.
type Failure = RepositoryError;

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_committed_rename_is_in_the_history"),
        word: Word::Succeeds,
        input: || Question::History,
        expect: Expect::Value(std::borrow::Cow::Borrowed("a.rs>b.rs")),
    },
    Case {
        name: CaseName("a_rename_in_the_working_tree_is_seen"),
        word: Word::Succeeds,
        input: || Question::Worktree,
        expect: Expect::Value(std::borrow::Cow::Borrowed("b.rs>c.rs")),
    },
    Case {
        name: CaseName("the_commit_that_added_an_operation_is_known"),
        word: Word::Succeeds,
        input: || Question::Introductions,
        expect: Expect::Value(std::borrow::Cow::Borrowed("1 root")),
    },
    Case {
        name: CaseName("the_lock_is_read_as_a_revision_holds_it"),
        word: Word::Succeeds,
        input: || Question::ReadAt,
        expect: Expect::Value(std::borrow::Cow::Borrowed("1")),
    },
    Case {
        name: CaseName("the_files_the_product_holds_are_listed"),
        word: Word::Succeeds,
        input: || Question::Files,
        expect: Expect::Value(std::borrow::Cow::Borrowed("c.rs")),
    },
    Case {
        name: CaseName("an_input_git_does_not_ignore_has_no_ignored_files"),
        word: Word::Succeeds,
        input: || Question::Ignored,
        expect: Expect::Value(std::borrow::Cow::Borrowed("0")),
    },
    Case {
        name: CaseName("a_commit_is_its_own_merge_base"),
        word: Word::Succeeds,
        input: || Question::MergeBase,
        expect: Expect::Value(std::borrow::Cow::Borrowed("head")),
    },
    Case {
        name: CaseName("a_revision_resolves_to_the_commit_it_names"),
        word: Word::Succeeds,
        input: || Question::Resolved,
        expect: Expect::Value(std::borrow::Cow::Borrowed("head")),
    },
    Case {
        name: CaseName("a_revision_no_commit_has_resolves_to_nothing"),
        word: Word::Refuses,
        input: || Question::UnknownCommit,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(RepositoryError::UnknownRevision {
                    revision: "no-such-revision".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_revision_no_commit_has_is_refused"),
        word: Word::Refuses,
        input: || Question::Unknown,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(RepositoryError::UnknownRevision {
                    revision: "no-such-revision".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// Runs the contract's rows against one implementation, over the repository it is bound to.
///
/// # Panics
///
/// When the implementation breaks the contract.
#[expect(
    clippy::panic,
    reason = "a contract check fails the adapter test that calls it by panicking, as the case runner it hands its rows to does; a repository with no head commit has no row to run"
)]
pub fn check(adapter: &impl RepositoryPort) {
    assert!(
        adapter
            .worktree()
            .is_ok_and(|listed| !listed.files.is_empty()),
        "the working tree of a repository with files is listed"
    );
    let Ok(Head::Commit(head)) = adapter.head() else {
        panic!("the repository has a head");
    };
    let subject = |question: Input| -> Result<String, Failure> {
        let renames = |list: &[crate::domain::renames::types::Rename]| {
            list.iter()
                .map(|rename| format!("{}>{}", rename.from.0, rename.to.0))
                .collect::<Vec<_>>()
                .join(" ")
        };
        // The lock's directory.
        let lock = Pathspec(".asymptote/operations".into());
        Ok(match question {
            Question::History => adapter
                .rename_history(&HistoryStart::Beginning, &head)?
                .0
                .iter()
                .map(|event| renames(&event.renames))
                .filter(|said| !said.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
            Question::Worktree => renames(&adapter.renames_to_worktree(&head)?.0),
            Question::Introductions => {
                let found = adapter.introductions(&head, &lock)?.0;
                let root_commit = found.iter().all(|introduction| {
                    introduction.parent == crate::domain::renames::types::Parent::Root
                });
                format!(
                    "{} {}",
                    found.len(),
                    if root_commit { "root" } else { "child" }
                )
            }
            Question::ReadAt => adapter.read_all_at(&head, &lock)?.0.len().to_string(),
            Question::Files => adapter
                .files(&[Pathspec("*.rs".into())])?
                .0
                .iter()
                .map(|path| path.0.to_string())
                .collect::<Vec<_>>()
                .join(" "),
            Question::Ignored => adapter
                .ignored(&[Pathspec("c.rs".into())])?
                .0
                .len()
                .to_string(),
            Question::MergeBase => if adapter.merge_base(&head, &head)? == head {
                "head"
            } else {
                "other"
            }
            .to_owned(),
            Question::Resolved => if adapter.commit(&Revision("HEAD".into()))? == head {
                "head"
            } else {
                "other"
            }
            .to_owned(),
            Question::UnknownCommit => adapter
                .commit(&Revision("no-such-revision".into()))?
                .0
                .to_string(),
            Question::Unknown => adapter
                .merge_base(&head, &Revision("no-such-revision".into()))?
                .0
                .to_string(),
        })
    };
    run(CASES, Word::Succeeds, |input| {
        subject(input).map(std::borrow::Cow::Owned)
    });
    run(CASES, Word::Refuses, |input| {
        subject(input).map(std::borrow::Cow::Owned)
    });
}
