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
    CorpusMemoryAdapter::new(vec![
        CorpusFile {
            path: FilePath("a.rs".into()),
            bytes: FileBytes(b"fn a() {}".to_vec()),
        },
        CorpusFile {
            path: FilePath("sub/b.rs".into()),
            bytes: FileBytes(b"fn b() {}\n".to_vec()),
        },
    ])
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Inventory => adapter().inventory().map(|entries| entries.0.len()),
        Scenario::Read(path) => {
            let entry = CorpusEntry {
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
