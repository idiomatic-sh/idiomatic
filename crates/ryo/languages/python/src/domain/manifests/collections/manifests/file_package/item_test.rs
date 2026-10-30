//! The `file_package` item's cases, every row in full.

use super::item::FilePackageItem;
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
                "README.md in pyproject.toml",
                "app.py in pyproject.toml",
                "packages/lib.two/pyproject.toml in packages/lib.two/pyproject.toml",
                "packages/lib_one/pyproject.toml in packages/lib_one/pyproject.toml",
                "packages/lib_one/src/lib_one/__init__.py in packages/lib_one/pyproject.toml",
                "packages/skip/pyproject.toml in packages/skip/pyproject.toml",
                "pyproject.toml in pyproject.toml",
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_monorepo_merges_pyproject_a_uv_workspace_and_requirements_dev"),
        word: Word::Succeeds,
        input: || MONOREPO,
        expect: Expect::Value(Reading {
            rows: &[
                "app.py in pyproject.toml",
                "packages/lib_one/pyproject.toml in packages/lib_one/pyproject.toml",
                "packages/lib_one/src/lib_one/__init__.py in packages/lib_one/pyproject.toml",
                "pyproject.toml in pyproject.toml",
                "requirements-dev.txt in pyproject.toml",
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_manifest_the_tool_cannot_read_leaves_the_relation_read_in_part"),
        word: Word::Succeeds,
        input: || WITH_A_BROKEN_MANIFEST,
        expect: Expect::Value(Reading {
            rows: &[
                "b/c.py in pyproject.toml",
                "b/pyproject.toml in pyproject.toml",
                "pyproject.toml in pyproject.toml",
            ],
            causes: &["b/pyproject.toml: unclosed table, expected `]`"],
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
