//! The `source_dependency` item's cases, every row in full: each import that names a package of the file's module, one row for each
//! file of it, and why the relation is read in part.

use super::item::SourceDependencyItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{IMPORTS, Reading, WITH_A_BROKEN_MANIFEST, WORKSPACE, reading};

/// The manifests and files of a repository: each file with its path and its text.
type Input = &'static [(&'static str, &'static str)];
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_import_of_a_package_of_the_module_is_a_row_to_each_of_its_files"),
        word: Word::Succeeds,
        input: || IMPORTS,
        expect: Expect::Value(Reading {
            rows: &[
                "cmd/tool/main.go > internal/store/sql.go at 21..53",
                "cmd/tool/main.go > internal/store/store.go at 21..53",
                "internal/store/store_test.go > internal/store/sql.go at 27..59",
                "internal/store/store_test.go > internal/store/store.go at 27..59",
                "main.go > internal/store/sql.go at 31..63",
                "main.go > internal/store/store.go at 31..63",
            ],
            causes: &[
                "`example.com/app/missing` names a package of the module that no directory of Go files is",
            ],
        }),
    },
    Case {
        name: CaseName("a_repository_whose_files_import_nothing_has_no_row"),
        word: Word::Succeeds,
        input: || WORKSPACE,
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName(
            "a_file_of_a_module_whose_go_mod_the_tool_cannot_read_leaves_the_relation_read_in_part",
        ),
        word: Word::Succeeds,
        input: || WITH_A_BROKEN_MANIFEST,
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &["b/go.mod declares no module the imports of its files could be linked to"],
        }),
    },
];

fn subject(files: Input) -> Value {
    reading::<SourceDependencyItem>(files)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
