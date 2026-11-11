//! Two file generation owners cannot reuse each other's results: each file's generation consults its own provider and keeps its output.
use super::fixtures::{Generation, Generations, two_generations};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{FileId, Value as Cell};
use std::borrow::Cow;

/// The files whose language columns the first and the second owner read.
type Input = (FileId, FileId);
type Value = Generations;
type Failure = OperatingError;

/// A fresh generation over one file: it consults its provider and emits the file.
const fn fresh(file: &'static [Cell]) -> Generation {
    Generation {
        read: true,
        cells: Cow::Borrowed(file),
        readiness: false,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("two_file_generation_owners_cannot_reuse_the_other_files_results"),
    word: Word::Succeeds,
    input: || (FileId(1), FileId(9)),
    expect: Expect::Value(Generations {
        first: fresh(&[Cell::File(FileId(1))]),
        second: fresh(&[Cell::File(FileId(9))]),
        // The transferred first output remains independent.
        first_cells_after_finish: Cow::Borrowed(&[Cell::File(FileId(1))]),
    }),
}];

/// Runs the two generation owners over their files.
fn subject((first, second): Input) -> Result<Value, Failure> {
    two_generations(first, second)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
