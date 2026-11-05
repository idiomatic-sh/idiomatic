//! The memory adapter of the corpus port: its cases, on files given as text.

use super::adapter::CorpusMemoryAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::fixtures::contract::check;
use crate::outbound::corpus::interface::CorpusPort as _;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{CorpusEntry, CorpusFile, FileBytes, FileSize, Nanoseconds};

/// What a row of the cases asks.
#[derive(Clone, Copy)]
enum Scenario {
    /// How many files the inventory lists, whatever root it is asked for.
    Inventory,
    /// How many bytes reading the file of this path brings in.
    Read(&'static str),
}

/// What a row of the cases asks.
type Input = Scenario;
/// A count.
type Value = usize;
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

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Inventory => adapter().inventory().map(|entries| entries.0.len()),
        Scenario::Read(path) => {
            let entry = CorpusEntry {
                executable: Default::default(),
                path: FilePath(path.into()),
                size: FileSize(0),
                modified: Nanoseconds(0),
            };
            let mut bytes = FileBytes::default();
            adapter().read(&entry, &mut bytes)?;
            Ok(bytes.0.len())
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_files_given_are_listed_whatever_the_root"),
        word: Word::Succeeds,
        input: || Scenario::Inventory,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_file_given_is_read_whole"),
        word: Word::Succeeds,
        input: || Scenario::Read("sub/b.rs"),
        expect: Expect::Value(10),
    },
    Case {
        name: CaseName("a_file_not_given_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Read("c.rs"),
        expect: Expect::Refusal(|error| selects(matches!(error, CorpusError::Unreadable { .. }))),
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

#[test]
fn given_inventory_distinguishes_absence_uncertainty_and_conflict() {
    use ryo_language::domain::project_context::types::{
        EntryObservation, EntryState, GivenProjectInventory, ProjectInventoryCompleteness,
    };
    let inventory = GivenProjectInventory {
        entries: vec![
            EntryObservation {
                path: FilePath("pkg".into()),
                state: EntryState::Directory,
            },
            EntryObservation {
                path: FilePath("conflict.py".into()),
                state: EntryState::Missing,
            },
            EntryObservation {
                path: FilePath("conflict.py".into()),
                state: EntryState::File,
            },
        ],
        completeness: ProjectInventoryCompleteness::CompleteWithinDeclaredScope,
    };
    let corpus = CorpusMemoryAdapter::new(
        vec![CorpusFile {
            executable: Default::default(),
            path: FilePath("real.py".into()),
            bytes: FileBytes(b"pass\n".to_vec()),
        }],
        inventory,
    );
    assert_eq!(
        corpus.observe_entry(&FilePath("pkg".into())).state,
        EntryState::Directory
    );
    assert_eq!(
        corpus.observe_entry(&FilePath("real.py".into())).state,
        EntryState::File
    );
    assert_eq!(
        corpus.observe_entry(&FilePath("absent.py".into())).state,
        EntryState::Missing
    );
    assert_eq!(
        corpus.observe_entry(&FilePath("conflict.py".into())).state,
        EntryState::Unknown
    );
}

#[test]
fn supplied_child_proves_parents_and_conflicting_inventory_stays_unknown() {
    use ryo_language::domain::project_context::types::{
        EntryObservation, EntryState, GivenProjectInventory, ProjectInventoryCompleteness,
    };
    let corpus = CorpusMemoryAdapter::new(
        vec![CorpusFile {
            executable: Default::default(),
            path: FilePath("src/pkg/mod.py".into()),
            bytes: FileBytes(b"value = 1\n".to_vec()),
        }],
        GivenProjectInventory {
            entries: vec![
                EntryObservation {
                    path: FilePath("src/pkg/mod.py".into()),
                    state: EntryState::Directory,
                },
                EntryObservation {
                    path: FilePath("src".into()),
                    state: EntryState::Missing,
                },
            ],
            completeness: ProjectInventoryCompleteness::CompleteWithinDeclaredScope,
        },
    );
    assert_eq!(
        corpus.observe_entry(&FilePath("src/pkg".into())).state,
        EntryState::Directory
    );
    assert_eq!(
        corpus.observe_entry(&FilePath("src".into())).state,
        EntryState::Unknown
    );
    assert_eq!(
        corpus
            .observe_entry(&FilePath("src/pkg/mod.py".into()))
            .state,
        EntryState::Unknown
    );
    assert_eq!(
        corpus
            .observe_entry(&FilePath("src/absent.py".into()))
            .state,
        EntryState::Missing
    );
}
