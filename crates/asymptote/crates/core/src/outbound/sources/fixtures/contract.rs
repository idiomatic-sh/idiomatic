//! What every `sources` adapter must pass: a file beneath the root is read, a path out of it is refused, and a
//! missing file is unreadable, and a file beneath the root is located inside it.

// Error locations are contractual; diagnostic wording belongs to each implementation.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::reports::types::{FileStat, FileState};
use crate::domain::violations::types::{Located, RepoPath};
use crate::outbound::sources::errors::SourcesError;
use crate::outbound::sources::interface::SourcesPort;

/// The path read.
type Input = &'static str;
/// Its text.
type Value = std::borrow::Cow<'static, str>;
/// The sources' error, which a refusal row expects by its path.
type Failure = SourcesError;

/// The contract's rows: every implementation runs them, over a root holding `src/a.rs` with the text `fn a() {}`.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_beneath_the_root_is_read"),
        word: Word::Succeeds,
        input: || "src/a.rs",
        expect: Expect::Value(std::borrow::Cow::Borrowed("fn a() {}")),
    },
    Case {
        name: CaseName("a_path_out_of_the_root_is_refused"),
        word: Word::Refuses,
        input: || "../a.rs",
        expect: Expect::Refusal(|actual| {
            if actual
                == &(SourcesError::Outside {
                    path: "../a.rs".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_absolute_path_is_refused"),
        word: Word::Refuses,
        input: || "/etc/hosts",
        expect: Expect::Refusal(|actual| {
            if actual
                == &(SourcesError::Outside {
                    path: "/etc/hosts".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_missing_file_is_unreadable"),
        word: Word::Fails,
        input: || "src/missing.rs",
        expect: Expect::Refusal(|error| {
            if matches!(error, SourcesError::Unreadable {path,reason} if path == "src/missing.rs" && !reason.is_empty())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// Runs the contract's rows against one implementation, over the product it is bound to.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl SourcesPort) {
    let subject = |path: Input| -> Result<String, Failure> {
        Ok(adapter.read(&RepoPath(path.into()))?.0.trim().to_owned())
    };
    run(CASES, Word::Succeeds, |input| {
        subject(input).map(std::borrow::Cow::Owned)
    });
    run(CASES, Word::Refuses, |input| {
        subject(input).map(std::borrow::Cow::Owned)
    });
    run(CASES, Word::Fails, |input| {
        subject(input).map(std::borrow::Cow::Owned)
    });
    let paths = [
        RepoPath("src/a.rs".into()),
        RepoPath("src/missing.rs".into()),
    ];
    let stats = adapter.stats(&paths);
    assert!(
        matches!(
            stats.files.as_slice(),
            [
                FileStat {
                    state: FileState::Present { .. },
                    ..
                },
                FileStat {
                    state: FileState::Absent,
                    ..
                }
            ]
        ),
        "a present file has a size and a time, a missing one is absent"
    );
    let digests = adapter.digests(&paths).0;
    assert_eq!(digests.len(), 2, "every path has a digest");
    assert_ne!(
        digests.first().map(|first| &first.digest),
        digests.get(1).map(|second| &second.digest),
        "a present file's digest is not an absent one's"
    );
    assert_eq!(
        adapter.locate(&RepoPath("src/a.rs".into())),
        Located::Inside,
        "a file beneath the root is located inside it"
    );
}
