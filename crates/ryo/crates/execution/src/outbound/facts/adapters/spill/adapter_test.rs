//! The spill adapter of the facts port: the port's contract rows run through it, then the cases specific to it: the words it spills
//! come back as they were kept, and a directory it cannot write in fails it.

use std::path::PathBuf;

use super::adapter::FactsSpillAdapter;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::selects;
use crate::outbound::facts::fixtures::contract::check;
use crate::outbound::facts::fixtures::words::test_words;
use crate::inbound::use_cases::execution::types::facts::FactsError;
use crate::inbound::use_cases::execution::types::facts::FactsPort as _;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_facts::types::PackedWords;

/// What a row of the cases asks of the adapter.
#[derive(Clone, Copy)]
enum Scenario {
    /// Keeps words of one bucket past the bound, so in the file, and reads them back at the finish.
    WordsComeBack,
    /// Keeps a few words: they come back from memory, and the directory holds no file.
    FewWordsStayInMemory,
    /// Keeps words past the bound in a directory that is not there.
    MissingDirectory,
}

/// What a row of the cases asks of the adapter.
type Input = Scenario;
/// A yes or no as a number.
type Value = usize;
type Failure = FactsError;

/// The rows of what is specific to this adapter: a name, one case word, an input from the fixtures or a constructor, and the expected
/// value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_words_kept_in_the_file_come_back_as_they_were_kept"),
        word: Word::Succeeds,
        input: || Scenario::WordsComeBack,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_few_words_come_back_from_memory_and_spill_nothing_to_disk"),
        word: Word::Succeeds,
        input: || Scenario::FewWordsStayInMemory,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("words_cannot_be_kept_in_a_directory_that_is_not_there"),
        word: Word::Fails,
        input: || Scenario::MissingDirectory,
        expect: Expect::Refusal(|error| selects!(matches!(error, FactsError::File { .. }))),
    },
];

/// Calls the adapter on one input, for its own rows.
fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::WordsComeBack => {
            let directory = tempfile::tempdir().expect("a directory");
            let adapter = FactsSpillAdapter::new(directory.path().to_path_buf());
            let mut worker = adapter.worker()?;
            let words = past_the_bound();
            worker.keep(Bucket(4), test_words(&words))?;
            worker.keep(Bucket(4), test_words(&PackedWords(vec![7, 8, 9])))?;
            let finished = adapter.finish(&mut vec![worker].into_iter())?;
            let kept = finished.kept(Bucket(4))?.words.0.0;
            Ok(usize::from(
                kept.len() == words.0.len() + 3 && kept.ends_with(&[7, 8, 9]),
            ))
        }
        Scenario::FewWordsStayInMemory => {
            let directory = tempfile::tempdir().expect("a directory");
            let adapter = FactsSpillAdapter::new(directory.path().to_path_buf());
            let mut worker = adapter.worker()?;
            worker.keep(Bucket(4), test_words(&PackedWords(vec![7, 8, 9])))?;
            let finished = adapter.finish(&mut vec![worker].into_iter())?;
            let empty = std::fs::read_dir(directory.path())
                .expect("the directory reads")
                .next()
                .is_none();
            Ok(usize::from(
                empty && finished.kept(Bucket(4))?.words.0.0 == [7, 8, 9],
            ))
        }
        Scenario::MissingDirectory => {
            let adapter =
                FactsSpillAdapter::new(PathBuf::from("/this/directory/is/missing/for/the/spill"));
            let mut worker = adapter.worker()?;
            worker
                .keep(Bucket(4), test_words(&past_the_bound()))
                .map(|()| 0)
        }
    }
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
