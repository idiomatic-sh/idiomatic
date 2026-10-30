//! The accepted and the refused inputs of the `native` adapter's schemas.

use super::{EntrySchema, FileSchema};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::paths::types::Entry;
use crate::domain::paths::types::EntryKind;
use crate::domain::paths::types::FileText;
use crate::domain::paths::types::Section;
use crate::outbound::repository::errors::RepositoryError;

/// What a row converts.
#[derive(Clone, Copy)]
enum Scenario {
    /// The bytes of a file that is text.
    Text,
    /// The bytes of a file that is not.
    Bytes,
    /// A directory entry with a name, and whether it is a directory.
    Entry(&'static str, bool),
}

/// What a row converts.
type Input = Scenario;
/// What the schema made of the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    /// A text.
    Text(&'static str),
    /// A directory entry: its name and whether it is a directory or a file.
    Entry(&'static str, EntryKind),
}
type Value = Answer;
type Failure = RepositoryError;

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Text => Section::try_from(FileSchema(b"abc".to_vec())).map(|text| match text {
            Section::Present(text) => Answer::Text(Box::leak(text.0)),
            Section::Absent => Answer::Text(""),
        }),
        Scenario::Bytes => {
            Section::<FileText>::try_from(FileSchema(vec![0xff, 0xfe])).map(|_| Answer::Text(""))
        }
        Scenario::Entry(name, directory) => {
            let held = tempfile::tempdir().expect("a directory");
            let path = held.path().join(name);
            if directory {
                std::fs::create_dir_all(&path).expect("a directory");
            } else {
                std::fs::write(&path, "").expect("a file");
            }
            let listed = std::fs::read_dir(held.path())
                .expect("a listing")
                .next()
                .expect("an entry")
                .expect("an entry that reads");
            Entry::try_from(EntrySchema(listed))
                .map(|entry| Answer::Entry(Box::leak(entry.name.0), entry.kind))
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_bytes_of_a_text_are_a_text"),
        word: Word::Succeeds,
        input: || Scenario::Text,
        expect: Expect::Value(Answer::Text("abc")),
    },
    Case {
        name: CaseName("a_directory_entry_says_whether_it_is_a_directory"),
        word: Word::Succeeds,
        input: || Scenario::Entry("sub", true),
        expect: Expect::Value(Answer::Entry("sub", EntryKind::Directory)),
    },
    Case {
        name: CaseName("a_file_entry_says_it_is_not"),
        word: Word::Succeeds,
        input: || Scenario::Entry("a.rs", false),
        expect: Expect::Value(Answer::Entry("a.rs", EntryKind::File)),
    },
    Case {
        name: CaseName("bytes_that_are_not_text_are_refused"),
        word: Word::Refuses,
        input: || Scenario::Bytes,
        expect: Expect::Error(RepositoryError::NotText),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
