//! What every `corpus` adapter must pass: only what every adapter of the port can promise, so a memory corpus, which has no root, passes
//! it as the disk does. What depends on a root being on the disk is the disk adapter's own test.

use super::super::errors::CorpusError;
use super::super::interface::CorpusPort;
use super::super::types::PathStat;
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{CorpusEntry, FileBytes, FileSize, Nanoseconds};

/// What a row of the contract asks.
#[derive(Clone, Copy, Debug)]
pub enum Scenario {
    /// Whether every file the inventory lists reads back with the size the inventory gave.
    ListedFilesReadBack,
    /// Reads a file the inventory does not list.
    AFileNotListed,
    /// Whether every file the inventory lists is stat'ed as the inventory lists it.
    ListedFilesStatAsListed,
    /// Whether a path where no file is is stat'ed as none.
    AMissingFileStatsAsNone,
}

/// What a row of the contract asks.
type Input = Scenario;
/// A yes or no as a number.
type Value = usize;
/// What a row of the contract expects when it is refused.
type Failure = CorpusError;

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_file_listed_reads_back_with_the_size_listed"),
        word: Word::Succeeds,
        input: || Scenario::ListedFilesReadBack,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("every_file_listed_stats_as_it_is_listed"),
        word: Word::Succeeds,
        input: || Scenario::ListedFilesStatAsListed,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_path_where_no_file_is_stats_as_none"),
        word: Word::Succeeds,
        input: || Scenario::AMissingFileStatsAsNone,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_file_that_is_not_listed_is_refused"),
        word: Word::Refuses,
        input: || Scenario::AFileNotListed,
        expect: Expect::Refusal(|error| {
            if matches!(error, CorpusError::Unreadable { .. }) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

/// Runs the contract's rows against one implementation, made over a repository that holds the files it should list.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl CorpusPort) {
    let subject = |scenario: Input| -> Result<Value, Failure> {
        match scenario {
            Scenario::ListedFilesReadBack => {
                let mut whole = true;
                for entry in adapter.inventory()?.0 {
                    let mut bytes = FileBytes::default();
                    adapter.read(&entry, &mut bytes)?;
                    whole &= u64::try_from(bytes.0.len()).ok() == Some(entry.size.0);
                }
                Ok(usize::from(whole))
            }
            Scenario::AFileNotListed => {
                let entry = CorpusEntry {
                    path: FilePath("this/file/is/not/listed.rs".into()),
                    size: FileSize(0),
                    modified: Nanoseconds(0),
                };
                adapter.read(&entry, &mut FileBytes::default()).map(|()| 0)
            }
            Scenario::ListedFilesStatAsListed => {
                let mut same = true;
                for entry in adapter.inventory()?.0 {
                    same &= adapter.stat(&entry.path)? == PathStat::Found(entry);
                }
                Ok(usize::from(same))
            }
            Scenario::AMissingFileStatsAsNone => Ok(usize::from(
                adapter.stat(&FilePath("this/file/is/not/there.rs".into()))? == PathStat::Missing,
            )),
        }
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
