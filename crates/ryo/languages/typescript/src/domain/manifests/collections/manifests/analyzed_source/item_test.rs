//! The `analyzed_source` item's cases, every row in full: each file the parse noted is examined.

use super::item::AnalyzedSourceItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{IMPORTS, WORKSPACE, read};

/// The manifests and files of a repository: each file with its path and its text.
type Input = &'static [(&'static str, &'static str)];
/// The number of each file examined, and how many causes made the relation read in part.
type Value = (&'static [u32], usize);
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_typescript_file_is_examined_whether_or_not_its_imports_name_a_file"),
        word: Word::Succeeds,
        input: || IMPORTS,
        expect: Expect::Value((&[0, 1, 2, 4, 6, 7, 8, 9], 0)),
    },
    Case {
        name: CaseName("a_manifest_is_not_examined"),
        word: Word::Succeeds,
        input: || WORKSPACE,
        expect: Expect::Value((&[0, 2], 0)),
    },
];

fn subject(files: Input) -> Value {
    let (written, _) = read::<AnalyzedSourceItem>(files);
    let rows: Vec<u32> = written.rows.iter().map(|row| row.path.0).collect();
    // The rows are leaked: a case compares them with rows that live for the whole run.
    (Box::leak(rows.into_boxed_slice()), written.partial)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
