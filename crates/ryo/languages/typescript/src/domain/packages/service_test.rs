//! Relative configuration paths normalize segments before package ownership.

use super::service::relative;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = (&'static str, &'static str, &'static str);
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("sibling_reference"),
        word: Word::Succeeds,
        input: || {
            (
                "packages/util",
                "../core/tsconfig.json",
                "packages/core/tsconfig.json",
            )
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("current_directory_reference"),
        word: Word::Succeeds,
        input: || {
            (
                "packages/core",
                "./tsconfig.json",
                "packages/core/tsconfig.json",
            )
        },
        expect: Expect::Value(true),
    },
];
fn subject((base, path, expected): Input) -> Value {
    relative(
        super::types::Directory(base),
        super::types::RepositoryPath(path),
    )
    .0
    .as_ref()
        == expected
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
