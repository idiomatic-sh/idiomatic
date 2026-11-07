//! An artifact pointer is its path and its digest together.
use crate::cases::{Case, CaseName, Expect, Word, run};
use super::value::ArtifactPointer;
use crate::domain::documents::types::text::value::{PathText, Sha256Text};

type Input = (&'static str, &'static str);
type Value = bool;
type Failure = ();
fn pointer((path, digest): Input) -> ArtifactPointer {
    ArtifactPointer {
        path: PathText(path.into()),
        sha256: Sha256Text(digest.into()),
    }
}
const CASES: &[Case<(Input, Input), Value, Failure>] = &[
    Case {
        name: CaseName("the same path and digest point at the same artifact"),
        word: Word::Succeeds,
        input: || (("out.bin", "a"), ("out.bin", "a")),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("another digest points at another artifact"),
        word: Word::Succeeds,
        input: || (("out.bin", "a"), ("out.bin", "b")),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("another path points at another artifact"),
        word: Word::Succeeds,
        input: || (("out.bin", "a"), ("err.bin", "a")),
        expect: Expect::Value(false),
    },
];
fn subject((left, right): (Input, Input)) -> Result<Value, Failure> {
    Ok(pointer(left) == pointer(right))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
