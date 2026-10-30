//! The `analyzed_source` item's cases, every row in full: each Python file the parse noted is examined.

use super::item::AnalyzedSourceItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{IMPORTS, Reading, WITH_A_BROKEN_MANIFEST, reading};

/// The manifests and files of a repository: each file with its path and its text.
type Input = &'static [(&'static str, &'static str)];
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_python_file_is_examined_whether_or_not_its_imports_name_a_file"),
        word: Word::Succeeds,
        input: || IMPORTS,
        expect: Expect::Value(Reading {
            rows: &[
                "app.py",
                "ns/part.py",
                "packages/lib_one/src/lib_one/__init__.py",
                "packages/lib_one/src/lib_one/workers.py",
                "tools/__init__.py",
                "tools/cli.py",
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_python_file_beside_a_manifest_the_tool_cannot_read_is_still_examined"),
        word: Word::Succeeds,
        input: || WITH_A_BROKEN_MANIFEST,
        expect: Expect::Value(Reading {
            rows: &["b/c.py"],
            causes: &[],
        }),
    },
];

fn subject(files: Input) -> Value {
    reading::<AnalyzedSourceItem>(files)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
