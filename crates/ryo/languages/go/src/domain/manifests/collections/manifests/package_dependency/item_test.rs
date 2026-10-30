//! The `package_dependency` item's cases, every row in full.

use super::item::PackageDependencyItem;
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
                "app/go.mod > libs/core/go.mod \"example.com/core\" \"example.com/core\" Normal \"\" Required Registry <\"v1.2.0\">",
                "app/go.mod > libs/util/go.mod \"example.com/util\" \"example.com/util\" Normal \"\" Required Registry <\"v0.1.0\">",
                "app/go.mod >  \"github.com/other/lib\" \"github.com/other/lib\" Normal \"\" Required Registry <\"v1.0.0\">",
                "libs/core/go.mod > libs/util/go.mod \"example.com/util\" \"example.com/util\" Normal \"\" Required Path <\"v0.1.0\"> replacement=\"../util\"",
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_manifest_the_tool_cannot_read_leaves_the_relation_read_in_part"),
        word: Word::Succeeds,
        input: || WITH_A_BROKEN_MANIFEST,
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &["b/go.mod: line 2: the `require (` block is not closed"],
        }),
    },
];

fn subject(files: Input) -> Value {
    reading::<PackageDependencyItem>(files)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
