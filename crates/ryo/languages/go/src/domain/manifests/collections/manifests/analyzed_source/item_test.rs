//! The `analyzed_source` item's cases, every row in full: each Go file the parse noted is examined, but one whose module's `go.mod`
//! was refused.

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
        name: CaseName(
            "every_go_file_is_examined_whether_or_not_its_imports_name_a_file_and_a_go_mod_is_no_source",
        ),
        word: Word::Succeeds,
        input: || IMPORTS,
        expect: Expect::Value(Reading {
            rows: &[
                "cmd/tool/main.go",
                "internal/store/sql.go",
                "internal/store/store.go",
                "internal/store/store_test.go",
                "main.go",
                "nested/lib/lib.go",
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_go_file_of_a_module_whose_go_mod_the_tool_cannot_read_is_not_examined"),
        word: Word::Succeeds,
        input: || WITH_A_BROKEN_MANIFEST,
        expect: Expect::Value(Reading {
            rows: &[],
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
