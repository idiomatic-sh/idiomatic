//! The `package_dependency` item's cases, every row in full.

use super::item::PackageDependencyItem;
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
                "packages/lib.two/pyproject.toml > packages/lib_one/pyproject.toml \"lib-one\" \"lib-one\" Normal \"sys_platform == 'win32'\" Required Registry",
                "packages/lib_one/pyproject.toml >  \"numpy\" \"numpy\" Normal \"\" Required Registry <\"==2.1.0\">!",
                "pyproject.toml > packages/lib_one/pyproject.toml \"Lib-One\" \"lib-one\" Normal \"\" Required Registry <\">=1.0\">",
                "pyproject.toml >  \"requests\" \"requests\" Normal \"python_version >= '3.9'\" Required Registry <\">=2\">",
                "pyproject.toml > packages/lib.two/pyproject.toml \"lib.two\" \"lib-two\" Normal \"\" Optional Registry",
                "pyproject.toml >  \"pytest\" \"pytest\" Dev \"\" Required Registry",
                "pyproject.toml > packages/lib_one/pyproject.toml \"lib-one\" \"lib-one\" Dev \"\" Required Registry",
                "pyproject.toml >  \"setuptools\" \"setuptools\" Build \"\" Required Registry <\">=61\">",
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
                "pyproject.toml > packages/lib_one/pyproject.toml \"lib-one\" \"lib-one\" Normal \"\" Required Registry <\">=1\">",
                "pyproject.toml >  \"pytest\" \"pytest\" Dev \"\" Required Registry",
                "pyproject.toml > packages/lib_one/pyproject.toml \"lib-one\" \"lib-one\" Dev \"sys_platform == 'win32'\" Required Registry",
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
            causes: &["b/pyproject.toml: unclosed table, expected `]`"],
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
