//! The `source_dependency` item's cases, every row in full: each import that names another file of the repository, and how many
//! imports make the relation read in part.

use super::item::SourceDependencyItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{IMPORTS, WORKSPACE, read};

/// The manifests and files of a repository: each file with its path and its text.
type Input = &'static [(&'static str, &'static str)];
/// Each row as its source's number, its target's, and where it starts and ends; then how many causes made the relation read in part.
type Value = (&'static [(u32, u32, u32, u32)], usize);
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_import_of_a_file_of_the_repository_is_a_row_to_it"),
        word: Word::Succeeds,
        input: || IMPORTS,
        expect: Expect::Value((
            &[
                (7, 6, 0, 24),
                (7, 0, 25, 57),
                (7, 1, 58, 85),
                (7, 9, 86, 119),
                (7, 8, 186, 208),
                (7, 2, 269, 295),
                (8, 7, 0, 29),
            ],
            2,
        )),
    },
    Case {
        name: CaseName("a_repository_whose_files_import_nothing_has_no_row"),
        word: Word::Succeeds,
        input: || WORKSPACE,
        expect: Expect::Value((&[], 0)),
    },
];

fn subject(files: Input) -> Value {
    let (written, _) = read::<SourceDependencyItem>(files);
    let rows: Vec<(u32, u32, u32, u32)> = written
        .rows
        .iter()
        .map(|row| (row.source.0, row.target.0, row.span.start.0, row.span.end.0))
        .collect();
    // The rows are leaked: a case compares them with rows that live for the whole run.
    (Box::leak(rows.into_boxed_slice()), written.partial)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
