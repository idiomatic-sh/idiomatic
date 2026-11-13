//! The spill adapter of the facts port: the port's contract rows run through it, then the cases specific to it: the words it spills
//! come back as they were kept, and a directory it cannot write in fails it.

use std::path::PathBuf;

use super::adapter::FactsSpillAdapter;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::fixtures::contract::check;
use crate::outbound::facts::fixtures::words::test_words;
use crate::outbound::facts::interface::FactsPort as _;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::selects;
use std::borrow::Cow;

/// The directory the adapter spills to.
#[derive(Clone, Copy, Debug)]
enum Directory {
    /// A fresh temporary directory.
    Temporary,
    /// A directory that is not there.
    Missing,
}

/// What one worker keeps in bucket 4 before the finish reads it back.
#[derive(Clone, Copy, Debug)]
struct Keep {
    directory: Directory,
    /// Whether it first keeps words past the bound the adapter writes to its file at.
    past_the_bound: bool,
    /// The words it keeps next.
    then: &'static [u8],
}

/// What the finish read back from bucket 4, and how many files the directory holds after it.
#[derive(Debug, PartialEq)]
struct Kept {
    len: usize,
    /// Whether the buffer is exactly the bucket's length: one read, no growth slack.
    exact_capacity: bool,
    /// The last three words.
    tail: Cow<'static, [u8]>,
    files: usize,
}

type Input = Keep;
type Value = Kept;
type Failure = FactsError;

/// The rows of what is specific to this adapter: a name, one case word, an input from the fixtures or a constructor, and the expected
/// value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_words_kept_in_the_file_come_back_once_into_a_buffer_of_their_length"),
        word: Word::Succeeds,
        input: || Keep {
            directory: Directory::Temporary,
            past_the_bound: true,
            then: &[7, 8, 9],
        },
        expect: Expect::Value(Kept {
            len: super::constants::FLUSH_AT + 3,
            exact_capacity: true,
            tail: Cow::Borrowed(&[7, 8, 9]),
            files: 0,
        }),
    },
    Case {
        name: CaseName("a_few_words_come_back_from_memory_and_spill_nothing_to_disk"),
        word: Word::Succeeds,
        input: || Keep {
            directory: Directory::Temporary,
            past_the_bound: false,
            then: &[7, 8, 9],
        },
        expect: Expect::Value(Kept {
            len: 3,
            exact_capacity: true,
            tail: Cow::Borrowed(&[7, 8, 9]),
            files: 0,
        }),
    },
    Case {
        name: CaseName("words_cannot_be_kept_in_a_directory_that_is_not_there"),
        word: Word::Fails,
        input: || Keep {
            directory: Directory::Missing,
            past_the_bound: true,
            then: &[],
        },
        expect: Expect::Refusal(|error| selects!(matches!(error, FactsError::File { .. }))),
    },
];

/// Keeps the words in one worker, finishes, and reads bucket 4 back.
fn subject(keep: Input) -> Result<Value, Failure> {
    let directory = tempfile::tempdir().expect("a directory");
    let path = match keep.directory {
        Directory::Temporary => directory.path().to_path_buf(),
        Directory::Missing => PathBuf::from("/this/directory/is/missing/for/the/spill"),
    };
    let adapter = FactsSpillAdapter::new(path);
    let worker = adapter.worker()?;
    if keep.past_the_bound {
        worker.keep(Bucket(4), test_words(&past_the_bound()))?;
    }
    worker.keep(Bucket(4), test_words(&PackedWords(keep.then.to_vec())))?;
    let finished = adapter.finish(&mut vec![worker].into_iter())?;
    let bucket = finished.kept(Bucket(4))?;
    let kept = &bucket.words.0.0;
    let files = std::fs::read_dir(directory.path())
        .expect("the directory reads")
        .count();
    Ok(Kept {
        len: kept.len(),
        exact_capacity: kept.capacity() == kept.len(),
        tail: kept[kept.len().saturating_sub(3)..].to_vec().into(),
        files,
    })
}

/// As many packed words as fill the bound past which the adapter writes them to its file: each a byte.
fn past_the_bound() -> PackedWords {
    PackedWords(vec![1; super::constants::FLUSH_AT])
}

#[test]
fn succeeds() {
    let directory = tempfile::tempdir().expect("a directory");
    check(&FactsSpillAdapter::new(directory.path().to_path_buf()));
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
