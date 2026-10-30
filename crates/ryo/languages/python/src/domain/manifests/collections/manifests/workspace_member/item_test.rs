//! The `workspace_member` item's cases, every row in full.

use super::item::WorkspaceMemberItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{MONOREPO, Reading, WITH_A_BROKEN_MANIFEST, WORKSPACE, reading};

/// The manifests and files of a repository: each file with its path and its text.
type Input = &'static [(&'static str, &'static str)];
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_workspace_and_its_packages"),
        word: Word::Succeeds,
        input: || WORKSPACE,
        expect: Expect::Value(Reading {
            rows: &[
                "pyproject.toml holds packages/lib.two/pyproject.toml",
                "pyproject.toml holds packages/lib_one/pyproject.toml",
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_monorepo_merges_pyproject_a_uv_workspace_and_requirements_dev"),
        word: Word::Succeeds,
        input: || MONOREPO,
        expect: Expect::Value(Reading {
            rows: &["pyproject.toml holds packages/lib_one/pyproject.toml"],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_manifest_the_tool_cannot_read_leaves_the_relation_read_in_part"),
        word: Word::Succeeds,
        input: || WITH_A_BROKEN_MANIFEST,
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &["b/pyproject.toml: unclosed table, expected `]`"],
        }),
    },
];

fn subject(files: Input) -> Value {
    reading::<WorkspaceMemberItem>(files)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
