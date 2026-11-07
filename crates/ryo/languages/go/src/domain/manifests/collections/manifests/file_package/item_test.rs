//! The `file_package` item's cases, every row in full.

use super::item::FilePackageItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{Reading, WITH_A_BROKEN_MANIFEST, WORKSPACE, reading};

/// The manifests and files of a repository: each file with its path and its text.
type Input = &'static [(&'static str, &'static str)];
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_workspace_and_its_modules"),
        word: Word::Succeeds,
        input: || WORKSPACE,
        expect: Expect::Value(Reading {
            rows: &[
                "app/go.mod in app/go.mod",
                "app/main.go in app/go.mod",
                "libs/core/core.go in libs/core/go.mod",
                "libs/core/go.mod in libs/core/go.mod",
                "libs/util/go.mod in libs/util/go.mod",
                "libs/util/util.go in libs/util/go.mod",
                "skip/go.mod in skip/go.mod",
                "skip/skip.go in skip/go.mod",
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_manifest_the_tool_cannot_read_leaves_the_relation_read_in_part"),
        word: Word::Succeeds,
        input: || WITH_A_BROKEN_MANIFEST,
        expect: Expect::Value(Reading {
            rows: &["b/c.go in go.mod", "b/go.mod in go.mod", "go.mod in go.mod"],
            causes: &["b/go.mod: line 2: the `require (` block is not closed"],
        }),
    },
];

fn subject(files: Input) -> Value {
    reading::<FilePackageItem>(files)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
