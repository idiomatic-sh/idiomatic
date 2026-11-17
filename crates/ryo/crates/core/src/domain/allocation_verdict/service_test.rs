//! The `allocation_verdict` service's cases, on values alone.

use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{CorpusFile, FileBytes};

use super::service::{differences, most};
use super::types::{AllocatedBlocks, Differences, InputBytes, JobBlocks, ParserBlocks};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Judges the files after the warm-up, not the first of each input.
    Judged,
    /// The most of differences of one block in a few files.
    FewGrew,
    /// The most of a difference of two blocks.
    TwoBlocks,
    /// The most of no file judged.
    NoneJudged,
}

/// What a row of the cases does.
type Input = Scenario;
/// The most blocks any judged file allocated beyond the parser library.
type Value = AllocatedBlocks;
type Failure = ();

fn file(path: &str) -> CorpusFile {
    CorpusFile {
        executable: Default::default(),
        path: FilePath(path.into()),
        bytes: FileBytes(b"x".to_vec()),
    }
}

fn each(blocks: &[u64]) -> Differences {
    Differences(blocks.iter().copied().map(AllocatedBlocks).collect())
}

fn subject(scenario: Scenario) -> Value {
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
            most(&judged)
        }
        Scenario::FewGrew => most(&each(&[0, 1, 0, 1, 1])),
        Scenario::TwoBlocks => most(&each(&[0, 2])),
        Scenario::NoneJudged => most(&each(&[])),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_first_file_of_each_input_and_the_warm_up_are_not_judged"),
        word: Word::Succeeds,
        input: || Scenario::Judged,
        expect: Expect::Value(AllocatedBlocks(1)),
    },
    Case {
        name: CaseName("the_most_is_one_where_a_few_files_allocate_one_block_beyond_the_parser"),
        word: Word::Succeeds,
        input: || Scenario::FewGrew,
        expect: Expect::Value(AllocatedBlocks(1)),
    },
    Case {
        name: CaseName("the_most_is_the_largest_difference"),
        word: Word::Succeeds,
        input: || Scenario::TwoBlocks,
        expect: Expect::Value(AllocatedBlocks(2)),
    },
    Case {
        name: CaseName("no_judged_file_allocates_nothing_beyond_the_parser"),
        word: Word::Succeeds,
        input: || Scenario::NoneJudged,
        expect: Expect::Value(AllocatedBlocks(0)),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
