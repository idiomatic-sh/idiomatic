//! The memory adapter of the corpus port: its cases, on files given as text.

use super::adapter::CorpusMemoryAdapter;
use crate::cases::{Case, CaseName, Expect, Matched, Word, run, selects};
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::fixtures::contract::check;
use crate::outbound::corpus::interface::CorpusPort as _;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::project_context::types::{
    EntryObservation, GivenProjectInventory, ObservedEntryState, ProjectInventoryCompleteness,
};
use ryo_values::domain::files::types::{CorpusEntry, CorpusFile, FileBytes, FileSize, Nanoseconds};

/// What a row of the cases asks.
#[derive(Clone, Copy)]
enum Scenario {
    /// How many files the inventory lists, whatever root it is asked for.
    Inventory,
    /// How many bytes reading the file of this path brings in.
    Read(&'static str),
    /// What observing the entry at this path says, over the corpus given.
    Observes(Given, &'static str),
}

/// The corpus a row observes entries of.
#[derive(Clone, Copy)]
enum Given {
    /// A directory, a missing file and a file given both ways beside a real file, in an inventory complete within its scope.
    Declared,
    /// A file below a directory the inventory calls a directory and a missing parent: the file proves its parents.
    SuppliedChild,
}

/// What a row of the cases answers.
#[derive(Debug, PartialEq)]
enum Seen {
    /// How many files, or how many bytes.
    Count(usize),
    /// What the entry observed is.
    State(ObservedEntryState),
}

/// What a row of the cases asks.
type Input = Scenario;
/// What a row of the cases answers.
type Value = Seen;
type Failure = CorpusError;

fn adapter() -> CorpusMemoryAdapter {
    CorpusMemoryAdapter::new(
        vec![
            CorpusFile {
                executable: Default::default(),
                path: FilePath("a.rs".into()),
                bytes: FileBytes(b"fn a() {}".to_vec()),
            },
            CorpusFile {
                executable: Default::default(),
                path: FilePath("sub/b.rs".into()),
                bytes: FileBytes(b"fn b() {}\n".to_vec()),
            },
        ],
        Default::default(),
    )
}

/// An inventory entry of this kind at this path.
fn entry(path: &str, state: ObservedEntryState) -> EntryObservation {
    EntryObservation {
        path: FilePath(path.into()),
        state,
    }
}

/// A corpus of one file given as text, over an inventory complete within its scope.
fn given(path: &str, entries: Vec<EntryObservation>) -> CorpusMemoryAdapter {
    CorpusMemoryAdapter::new(
        vec![CorpusFile {
            executable: Default::default(),
            path: FilePath(path.into()),
            bytes: FileBytes(b"pass\n".to_vec()),
        }],
        GivenProjectInventory {
            entries,
            completeness: ProjectInventoryCompleteness::CompleteWithinDeclaredScope,
        },
    )
}

fn corpus(given_corpus: Given) -> CorpusMemoryAdapter {
    match given_corpus {
        Given::Declared => given(
            "real.py",
            vec![
                entry("pkg", ObservedEntryState::Directory),
                entry("conflict.py", ObservedEntryState::Missing),
                entry("conflict.py", ObservedEntryState::File),
            ],
        ),
        Given::SuppliedChild => given(
            "src/pkg/mod.py",
            vec![
                entry("src/pkg/mod.py", ObservedEntryState::Directory),
                entry("src", ObservedEntryState::Missing),
            ],
        ),
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Observes(which, path) => Ok(Seen::State(
            corpus(which).observe_entry(&FilePath(path.into())).state,
        )),
        Scenario::Inventory => adapter()
            .inventory()
            .map(|entries| Seen::Count(entries.0.len())),
        Scenario::Read(path) => {
            let entry = CorpusEntry {
                executable: Default::default(),
                path: FilePath(path.into()),
                size: FileSize(0),
                modified: Nanoseconds(0),
            };
            let mut bytes = FileBytes::default();
            adapter().read(&entry, &mut bytes)?;
            Ok(Seen::Count(bytes.0.len()))
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_files_given_are_listed_whatever_the_root"),
        word: Word::Succeeds,
        input: || Scenario::Inventory,
        expect: Expect::Value(Seen::Count(2)),
    },
    Case {
        name: CaseName("a_file_given_is_read_whole"),
        word: Word::Succeeds,
        input: || Scenario::Read("sub/b.rs"),
        expect: Expect::Value(Seen::Count(10)),
    },
    Case {
        name: CaseName("a_directory_the_inventory_gives_is_observed_as_one"),
        word: Word::Succeeds,
        input: || Scenario::Observes(Given::Declared, "pkg"),
        expect: Expect::Value(Seen::State(ObservedEntryState::Directory)),
    },
    Case {
        name: CaseName("a_file_given_as_text_is_observed_as_a_file"),
        word: Word::Succeeds,
        input: || Scenario::Observes(Given::Declared, "real.py"),
        expect: Expect::Value(Seen::State(ObservedEntryState::File)),
    },
    Case {
        name: CaseName("a_path_absent_from_a_complete_inventory_is_observed_as_missing"),
        word: Word::Succeeds,
        input: || Scenario::Observes(Given::Declared, "absent.py"),
        expect: Expect::Value(Seen::State(ObservedEntryState::Missing)),
    },
    Case {
        name: CaseName("a_path_the_inventory_gives_two_states_is_observed_as_unknown"),
        word: Word::Succeeds,
        input: || Scenario::Observes(Given::Declared, "conflict.py"),
        expect: Expect::Value(Seen::State(ObservedEntryState::Unknown)),
    },
    Case {
        name: CaseName("a_file_given_proves_the_directory_above_it"),
        word: Word::Succeeds,
        input: || Scenario::Observes(Given::SuppliedChild, "src/pkg"),
        expect: Expect::Value(Seen::State(ObservedEntryState::Directory)),
    },
    Case {
        name: CaseName("a_parent_the_inventory_calls_missing_is_observed_as_unknown"),
        word: Word::Succeeds,
        input: || Scenario::Observes(Given::SuppliedChild, "src"),
        expect: Expect::Value(Seen::State(ObservedEntryState::Unknown)),
    },
    Case {
        name: CaseName("a_file_the_inventory_calls_a_directory_is_observed_as_unknown"),
        word: Word::Succeeds,
        input: || Scenario::Observes(Given::SuppliedChild, "src/pkg/mod.py"),
        expect: Expect::Value(Seen::State(ObservedEntryState::Unknown)),
    },
    Case {
        name: CaseName("a_path_absent_below_a_proven_parent_is_observed_as_missing"),
        word: Word::Succeeds,
        input: || Scenario::Observes(Given::SuppliedChild, "src/absent.py"),
        expect: Expect::Value(Seen::State(ObservedEntryState::Missing)),
    },
    Case {
        name: CaseName("a_file_not_given_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Read("c.rs"),
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(error, CorpusError::Unreadable { .. })))
        }),
    },
];

#[test]
fn succeeds() {
    check(&adapter());
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
