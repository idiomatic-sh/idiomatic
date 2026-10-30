//! The `source_dependency` item's cases, every row in full: each import that names a module a file of the repository is, and why the
//! relation is read in part.

use super::item::SourceDependencyItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{IMPORTS, Reading, WORKSPACE, reading};

/// The manifests and files of a repository: each file with its path and its text.
type Input = &'static [(&'static str, &'static str)];
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_import_of_a_module_of_the_repository_is_a_row_to_its_file"),
        word: Word::Succeeds,
        input: || IMPORTS,
        expect: Expect::Value(Reading {
            rows: &[
                "app.py > packages/lib_one/src/lib_one/__init__.py at 17..24",
                "app.py > packages/lib_one/src/lib_one/workers.py at 45..52",
                "app.py > packages/lib_one/src/lib_one/__init__.py at 73..76",
                "app.py > ns/part.py at 97..102",
                "app.py > tools/cli.py at 143..146",
                "packages/lib_one/src/lib_one/__init__.py > packages/lib_one/src/lib_one/workers.py at 21..24",
                "packages/lib_one/src/lib_one/__init__.py > packages/lib_one/src/lib_one/workers.py at 39..46",
                "tools/cli.py > app.py at 7..10",
            ],
            causes: &[
                "`lib_one.absent` names a module of the repository that no file is",
                "`..nowhere` names a module of the repository that no file is",
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
];

fn subject(files: Input) -> Value {
    reading::<SourceDependencyItem>(files)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
