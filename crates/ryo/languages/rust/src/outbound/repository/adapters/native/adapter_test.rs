//! The `native` adapter of the repository port: the port's contract rows run through it, over files written for the purpose, then the
//! cases specific to it.

use super::adapter::RepositoryNativeAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::paths::service::present;
use crate::domain::paths::types::RepoPath;
use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::repository::factory::native;
use crate::outbound::repository::fixtures::contract::{FILES, check};
use crate::outbound::repository::interface::RepositoryPort as _;
use ryo::languages::RootPath;

/// What a row of the cases asks of the files.
#[derive(Clone, Copy)]
enum Scenario {
    /// The text of a file that is not text.
    Binary,
    /// The text of a file that is not there.
    Missing,
    /// What a directory that is not there holds.
    NoDirectory,
}

/// What a row of the cases asks.
type Input = Scenario;
/// What came back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    /// The text of a file, when it has one.
    Text(Option<&'static str>),
    /// The names of what a directory holds.
    Entries(&'static [&'static str]),
}
type Value = Answer;
type Failure = RepositoryError;

/// A directory holding the contract's files and a file that is not text.
fn written() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a directory");
    for (path, text) in FILES {
        let place = root.path().join(path);
        std::fs::create_dir_all(place.parent().expect("a parent")).expect("a directory");
        std::fs::write(place, text).expect("a file");
    }
    std::fs::write(root.path().join("binary"), [0xff_u8, 0xfe]).expect("a file");
    root
}

fn subject(scenario: Input) -> Value {
    let root = written();
    let adapter = RepositoryNativeAdapter::new(RootPath(root.path().to_string_lossy().into()));
    match scenario {
        Scenario::Binary => Answer::Text(
            present(adapter.text(&RepoPath("binary".into()))).map(|text| &*Box::leak(text.0)),
        ),
        Scenario::Missing => Answer::Text(
            present(adapter.text(&RepoPath("missing".into()))).map(|text| &*Box::leak(text.0)),
        ),
        Scenario::NoDirectory => {
            let listed: Vec<&'static str> = adapter
                .entries(&RepoPath("missing".into()))
                .0
                .into_iter()
                .map(|entry| &*Box::leak(entry.name.0))
                .collect();
            Answer::Entries(Box::leak(listed.into_boxed_slice()))
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_that_is_not_text_has_no_text"),
        word: Word::Succeeds,
        input: || Scenario::Binary,
        expect: Expect::Value(Answer::Text(None)),
    },
    Case {
        name: CaseName("a_file_that_is_not_there_has_no_text"),
        word: Word::Succeeds,
        input: || Scenario::Missing,
        expect: Expect::Value(Answer::Text(None)),
    },
    Case {
        name: CaseName("a_directory_that_is_not_there_holds_nothing"),
        word: Word::Succeeds,
        input: || Scenario::NoDirectory,
        expect: Expect::Value(Answer::Entries(&[])),
    },
];

#[test]
fn succeeds() {
    let root = written();
    check(&native(RootPath(root.path().to_string_lossy().into())));
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
