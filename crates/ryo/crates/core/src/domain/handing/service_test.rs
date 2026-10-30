//! The `handing` service's cases, on values alone: what a file handed is put in the store's tables and given back from them.

use std::convert::Infallible;

use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{FileSize, Nanoseconds};
use ryo_values::domain::records::types::{GoverningRules, Handed, HandedNotes, HandedWords};
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stored_facts::types::{
    ArenaBytes, Bucket, Digest, FileNotes, LanguageNumber, NoteBytes,
};
use ryo_values::domain::stores::types::{
    BucketsWanted, Stored, StoredFile, StoredFileNumber, StoredHanded,
};

use super::service::{hand_into, handed_in, rows_of_file};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// A scenario: what a row asks of the part.
type Input = fn() -> Value;
/// What a scenario answers: a yes as one, or a count.
type Value = usize;
/// Nothing of this part is refused.
type Failure = Infallible;

/// A stored file at `path`, its digests blank.
fn file(path: &str) -> StoredFile {
    StoredFile {
        path: FilePath(path.into()),
        digest: Digest([0; 16]),
        interface: Digest([0; 16]),
        size: FileSize(0),
        modified: Nanoseconds(0),
        noted: Digest([0; 16]),
    }
}

/// The store of two files, `a.rs` and `b.rs`.
fn stored() -> Stored {
    Stored {
        files: vec![file("a.rs"), file("b.rs")],
        ..Stored::default()
    }
}

/// What a file hands: words in the buckets 1 and 2, a text, notes and the rules that govern it.
fn handed() -> Handed {
    Handed {
        buckets: vec![
            HandedWords {
                bucket: Bucket(1),
                words: PackedWords(vec![1]),
            },
            HandedWords {
                bucket: Bucket(2),
                words: PackedWords(vec![2]),
            },
        ],
        text: ArenaBytes(b"name".to_vec()),
        notes: HandedNotes::Noted(FileNotes {
            language: LanguageNumber(0),
            notes: NoteBytes(vec![7]),
        }),
        governing: GoverningRules(vec![0]),
        partials: Vec::new(),
    }
}

fn gives_back_what_the_file_handed() -> Value {
    let mut tables = StoredHanded::default();
    hand_into(
        &mut tables,
        StoredFileNumber(1),
        handed(),
        &BucketsWanted::All,
    );
    // Its words stay where the store keeps them.
    usize::from(
        handed_in(&stored(), &tables, &FilePath("b.rs".into()))
            == Handed {
                buckets: Vec::new(),
                ..handed()
            },
    )
}

fn gives_nothing_for_a_file_the_store_has_no_row_of() -> Value {
    let mut tables = StoredHanded::default();
    hand_into(
        &mut tables,
        StoredFileNumber(1),
        handed(),
        &BucketsWanted::All,
    );
    usize::from(handed_in(&stored(), &tables, &FilePath("c.rs".into())) == Handed::default())
}

fn keeps_only_the_buckets_rebuilt() -> Value {
    let mut tables = StoredHanded::default();
    hand_into(
        &mut tables,
        StoredFileNumber(0),
        handed(),
        &BucketsWanted::These(vec![Bucket(2)]),
    );
    tables.frontier.len()
}

fn finds_the_rows_of_one_file() -> Value {
    let rows = [0, 1, 1, 2].map(StoredFileNumber);
    rows_of_file(&rows, StoredFileNumber(1), |each| *each).len()
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("what_a_file_handed_is_given_back_from_the_store"),
        word: Word::Succeeds,
        input: || gives_back_what_the_file_handed,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_file_the_store_has_no_row_of_handed_nothing"),
        word: Word::Succeeds,
        input: || gives_nothing_for_a_file_the_store_has_no_row_of,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("only_the_buckets_rebuilt_are_kept"),
        word: Word::Succeeds,
        input: || keeps_only_the_buckets_rebuilt,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_rows_of_one_file_are_found_together"),
        word: Word::Succeeds,
        input: || finds_the_rows_of_one_file,
        expect: Expect::Value(2),
    },
];

fn subject(scenario: Input) -> Value {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |scenario| Ok(subject(scenario)));
}
