//! The `allocation_verdict` service's cases, on values alone.

use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{CorpusFile, FileBytes};

use super::service::{differences, verdict};
use super::types::{
    AllocatedBlocks, AllocationVerdict, Differences, FileCount, InputBytes, JobBlocks, ParserBlocks,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::corpus_copies::types::CopyCount;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Judges the files after the warm-up, not the first of each input.
    Judged,
    /// Judges differences of one block in a few files.
    FewGrew,
    /// Judges a difference of two blocks.
    TwoBlocks,
    /// Judges one block in more files than the doublings explain.
    ManyGrew,
}

/// What a row of the cases does.
type Input = Scenario;
/// The verdict on the differences.
type Value = AllocationVerdict;
type Failure = ();

fn file(path: &str) -> CorpusFile {
    CorpusFile {
        path: FilePath(path.into()),
        bytes: FileBytes(b"x".to_vec()),
    }
}

fn each(blocks: &[u64]) -> Differences {
    Differences(blocks.iter().copied().map(AllocatedBlocks).collect())
}

fn subject(scenario: Scenario) -> Value {
    let copies = CopyCount(60);
    match scenario {
        Scenario::Judged => {
            let files: Vec<CorpusFile> = ["a/1", "a/2", "a/3", "b/1", "b/2", "b/3"]
                .into_iter()
                .map(file)
                .collect();
            let parser = ParserBlocks(
                [(InputBytes(b"x".to_vec()), AllocatedBlocks(2))]
                    .into_iter()
                    .collect(),
            );
            let judged = differences(&JobBlocks(vec![9, 9, 9, 9, 2, 3]), &files, &parser);
            verdict(&judged, copies)
        }
        Scenario::FewGrew => verdict(&each(&[0, 1, 0, 1, 1]), copies),
        Scenario::TwoBlocks => verdict(&each(&[0, 2]), copies),
        Scenario::ManyGrew => verdict(&each(&[1; 20]), copies),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_first_file_of_each_input_and_the_warm_up_are_not_judged"),
        word: Word::Succeeds,
        input: || Scenario::Judged,
        expect: Expect::Value(AllocationVerdict::Within),
    },
    Case {
        name: CaseName("a_few_files_one_block_beyond_the_parser_are_within"),
        word: Word::Succeeds,
        input: || Scenario::FewGrew,
        expect: Expect::Value(AllocationVerdict::Within),
    },
    Case {
        name: CaseName("two_blocks_beyond_the_parser_is_too_many"),
        word: Word::Succeeds,
        input: || Scenario::TwoBlocks,
        expect: Expect::Value(AllocationVerdict::BeyondOneBlock),
    },
    Case {
        name: CaseName("more_files_than_the_doublings_explain_is_too_many"),
        word: Word::Succeeds,
        input: || Scenario::ManyGrew,
        expect: Expect::Value(AllocationVerdict::TooManyGrew {
            grew: FileCount(20),
            judged: FileCount(20),
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
